//! The test registrar and test user of one suite run, and the contracts the
//! registrar owns.
//!
//! The registrar is a plain party. It builds the same utility stack that the
//! sandbox bootstrap builds for BETH: a provider service, a user service, a
//! provider credential and a registrar credential, and a registrar service
//! with its allocation factory. That part is shared by every asset. Each
//! asset then adds its own instrument configuration and rules, as an
//! `AssetStack`.

use std::marker::PhantomData;

use serde_json::json;

use crate::{
    InstrumentId,
    flows::canton_bridge_v1::CantonBridgeV1,
    kits::{
        bitsafe_api::{AccountContractRuleSet, ContractInfo, TokenStandardContracts},
        canton::create_args,
    },
    localnet::ledger::{Ledger, USER_ID, contract_info, created_cid},
};

pub(crate) const GOVERNANCE_RULES: &str = "#governance-core-v1:Governance.Rules:GovernanceRules";
pub(crate) const INSTRUMENT_CONFIGURATION: &str =
    "#utility-registry-v0:Utility.Registry.V0.Configuration.Instrument:InstrumentConfiguration";

const OPERATOR_CONFIGURATION: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Configuration.Operator:OperatorConfiguration";
const PROVIDER_CONFIGURATION: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Configuration.Provider:ProviderConfiguration";
const PROVIDER_SERVICE_REQUEST: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Service.Provider:ProviderServiceRequest";
const PROVIDER_SERVICE: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Service.Provider:ProviderService";
const REGISTRAR_SERVICE_REQUEST: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Service.Registrar:RegistrarServiceRequest";
const REGISTRAR_SERVICE: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Service.Registrar:RegistrarService";
const ALLOCATION_FACTORY: &str =
    "#utility-registry-app-v0:Utility.Registry.App.V0.Service.AllocationFactory:AllocationFactory";
const USER_SERVICE_REQUEST: &str =
    "#utility-credential-app-v0:Utility.Credential.App.V0.Service.User:UserServiceRequest";
const USER_SERVICE: &str =
    "#utility-credential-app-v0:Utility.Credential.App.V0.Service.User:UserService";
const CREDENTIAL_OFFER: &str =
    "#utility-credential-app-v0:Utility.Credential.App.V0.Model.Offer:CredentialOffer";
const CREDENTIAL: &str = "#utility-credential-v0:Utility.Credential.V0.Credential:Credential";

/// The parties and the asset-neutral contracts of one suite run.
pub(crate) struct Fixture {
    pub(crate) ledger: Ledger,
    /// The fresh plain party that plays the registrar of every asset.
    pub(crate) registrar: String,
    /// The fresh party that owns the accounts and holdings.
    pub(crate) user: String,
    /// The one-member governance body of `registrar`.
    pub(crate) governance_rules: String,
    /// The registrar's user service. The registrar offers credentials
    /// through it.
    pub(crate) user_service: String,
    /// The registrar's registrar service, which configures instruments.
    registrar_service: String,
    /// The registrar's allocation factory, which mints and burns every
    /// instrument the registrar configures.
    pub(crate) allocation_factory: ContractInfo,
    operator: String,
}

/// The contracts of one asset: its instrument configuration and its rules.
pub(crate) struct AssetStack<A> {
    pub(crate) instrument_configuration: ContractInfo,
    pub(crate) deposit_rules: ContractInfo,
    pub(crate) withdraw_rules: ContractInfo,
    allocation_factory: ContractInfo,
    registrar: String,
    _asset: PhantomData<A>,
}

impl Fixture {
    /// Finds the sandbox's operator and dso, allocates a fresh registrar and
    /// user, and gives the registrar a governance body with itself as the
    /// only member. Then the registrar builds the shared utility stack.
    pub(crate) async fn new() -> Result<Fixture, String> {
        // The library's own writes send this user id. A second `set` fails,
        // and the value is the same, so the result does not matter.
        let _ = crate::kits::canton::TEST_USER_ID.set(USER_ID.to_string());
        let ledger = Ledger::from_env();
        let operator = ledger.party("operator").await?;
        let dso = ledger.party("dso").await?;
        let registrar = ledger.allocate_party("testreg").await?;
        let user = ledger.allocate_party("testuser").await?;
        let r = registrar.as_str();
        let o = operator.as_str();
        // Daml's `Set Party` is a map from party to unit, `Int` is a string,
        // and `RelTime` is a count of microseconds.
        let governance_rules = ledger
            .create(
                &[r],
                GOVERNANCE_RULES,
                json!({
                    "governanceParty": r,
                    "members": {"map": [[r, {}]]},
                    "threshold": "1",
                    "actionConfirmationTimeout": {"microseconds": "1800000000"},
                    "additionalProposers": null,
                }),
            )
            .await?;

        // The registrar asks the operator for a provider service and a
        // user service.
        let provider_service_request = ledger
            .create(
                &[r],
                PROVIDER_SERVICE_REQUEST,
                json!({"operator": o, "provider": r}),
            )
            .await?;
        let user_service_request = ledger
            .create(
                &[r],
                USER_SERVICE_REQUEST,
                json!({"operator": o, "user": r}),
            )
            .await?;

        // The operator offers the registrar the provider credential through
        // the operator's own user service. The operator also holds user
        // services for other parties.
        let operator_user_service = ledger
            .active(o, USER_SERVICE)
            .await?
            .into_iter()
            .find(|contract| {
                create_args(contract).is_ok_and(|args| args.get("user") == Some(&json!(o)))
            })
            .ok_or("The operator has no user service of its own")?
            .created_event
            .contract_id;
        let response = ledger
            .exercise(
                &[o],
                USER_SERVICE,
                &operator_user_service,
                "UserService_OfferFreeCredential",
                credential_offer(r, "provider", "Provider"),
                &[],
            )
            .await?;
        let provider_offer = created_cid(&response, CREDENTIAL_OFFER)?;

        // The operator accepts the user service request; the registrar
        // accepts the provider credential.
        let response = ledger
            .exercise(
                &[o],
                USER_SERVICE_REQUEST,
                &user_service_request,
                "UserServiceRequest_Accept",
                json!({"dso": dso}),
                &[],
            )
            .await?;
        let user_service = created_cid(&response, USER_SERVICE)?;
        let response = ledger
            .exercise(
                &[r],
                CREDENTIAL_OFFER,
                &provider_offer,
                "CredentialOffer_AcceptFree",
                json!({}),
                &[],
            )
            .await?;
        let provider_credential = created_cid(&response, CREDENTIAL)?;

        // The operator accepts the provider service request on the
        // strength of the provider credential.
        let operator_configuration = ledger
            .active(o, OPERATOR_CONFIGURATION)
            .await?
            .into_iter()
            .next()
            .ok_or("The operator has no operator configuration")?
            .created_event
            .contract_id;
        let response = ledger
            .exercise(
                &[o],
                PROVIDER_SERVICE_REQUEST,
                &provider_service_request,
                "ProviderServiceRequest_Accept",
                json!({
                    "operatorConfigurationCid": operator_configuration,
                    "credentialCids": [provider_credential],
                    "appRewardConfigurationDetails": {
                        "dso": dso,
                        "operatorAppRewardBeneficiary": {"beneficiary": o, "weight": "1.0"},
                    },
                }),
                &[],
            )
            .await?;
        let provider_service = created_cid(&response, PROVIDER_SERVICE)?;

        // The registrar requires a registrar credential from itself, and
        // issues that credential to itself.
        let response = ledger
            .exercise(
                &[r],
                PROVIDER_SERVICE,
                &provider_service,
                "ProviderService_CreateProviderConfiguration",
                json!({
                    "registrarRequirements": [{
                        "issuer": r,
                        "requiredClaims": [{"_1": "hasRegistryRole", "_2": "Registrar"}],
                    }],
                    "holderRequirements": [],
                }),
                &[],
            )
            .await?;
        let provider_configuration = created_cid(&response, PROVIDER_CONFIGURATION)?;
        let response = ledger
            .exercise(
                &[r],
                USER_SERVICE,
                &user_service,
                "UserService_OfferFreeCredential",
                credential_offer(r, "registrar", "Registrar"),
                &[],
            )
            .await?;
        let registrar_offer = created_cid(&response, CREDENTIAL_OFFER)?;
        let response = ledger
            .exercise(
                &[r],
                CREDENTIAL_OFFER,
                &registrar_offer,
                "CredentialOffer_AcceptFree",
                json!({}),
                &[],
            )
            .await?;
        let registrar_credential = created_cid(&response, CREDENTIAL)?;

        // The registrar opens its registrar service, which creates the
        // allocation factory.
        let registrar_service_request = ledger
            .create(
                &[r],
                REGISTRAR_SERVICE_REQUEST,
                json!({
                    "operator": o,
                    "provider": r,
                    "registrar": r,
                    "createTransferRule": true,
                    "createAllocationFactory": true,
                }),
            )
            .await?;
        let response = ledger
            .exercise(
                &[r],
                PROVIDER_SERVICE,
                &provider_service,
                "ProviderService_AcceptRegistrarServiceRequest",
                json!({
                    "cid": registrar_service_request,
                    "payload": {
                        "providerConfigurationCid": provider_configuration,
                        "credentialCids": [provider_credential, registrar_credential],
                    },
                }),
                &[],
            )
            .await?;
        let registrar_service = created_cid(&response, REGISTRAR_SERVICE)?;
        let allocation_factory = created_cid(&response, ALLOCATION_FACTORY)?;
        let allocation_factory =
            read_back(&ledger, r, ALLOCATION_FACTORY, &allocation_factory).await?;
        Ok(Fixture {
            ledger,
            registrar,
            user,
            governance_rules,
            user_service,
            registrar_service,
            allocation_factory,
            operator,
        })
    }

    /// The registrar configures the instrument of asset `A` and creates its
    /// deposit and withdraw rules.
    pub(crate) async fn asset_stack<A: CantonBridgeV1>(&self) -> Result<AssetStack<A>, String> {
        let r = self.registrar.as_str();
        let response = self
            .ledger
            .exercise(
                &[r],
                REGISTRAR_SERVICE,
                &self.registrar_service,
                "RegistrarService_CreateInstrumentConfiguration",
                json!({
                    "instrumentId": A::TICKER,
                    "additionalIdentifiers": [{
                        "source": r,
                        "id": A::TICKER,
                        "scheme": "RegistrarInternalScheme",
                    }],
                    "issuerRequirements": [],
                    "holderRequirements": [],
                }),
                &[],
            )
            .await?;
        let instrument_configuration = created_cid(&response, INSTRUMENT_CONFIGURATION)?;
        let rules = json!({
            "registrar": r,
            "operator": self.operator,
            "instrument": {"admin": r, "id": A::TICKER},
        });
        let deposit_rules = self
            .ledger
            .create(&[r], A::DEPOSIT_ACCOUNT_RULES, rules.clone())
            .await?;
        let withdraw_rules = self
            .ledger
            .create(&[r], A::WITHDRAW_ACCOUNT_RULES, rules)
            .await?;
        Ok(AssetStack {
            instrument_configuration: read_back(
                &self.ledger,
                r,
                INSTRUMENT_CONFIGURATION,
                &instrument_configuration,
            )
            .await?,
            deposit_rules: read_back(&self.ledger, r, A::DEPOSIT_ACCOUNT_RULES, &deposit_rules)
                .await?,
            withdraw_rules: read_back(&self.ledger, r, A::WITHDRAW_ACCOUNT_RULES, &withdraw_rules)
                .await?,
            allocation_factory: self.allocation_factory.clone(),
            registrar: self.registrar.clone(),
            _asset: PhantomData,
        })
    }

    /// The registrar offers the test user the Minter claim of asset `A`
    /// through its user service. Returns the cid of the credential offer.
    pub(crate) async fn offer_minter_credential<A: CantonBridgeV1>(
        &self,
    ) -> Result<String, String> {
        let (property, value) = A::MINTER_CLAIM;
        let response = self
            .ledger
            .exercise(
                &[&self.registrar],
                USER_SERVICE,
                &self.user_service,
                "UserService_OfferFreeCredential",
                json!({
                    "holder": self.user,
                    "id": format!("localnet-{}-minter-credential", A::API_PATH),
                    "description": format!("localnet {} minter", A::TICKER),
                    "claims": [{"subject": self.user, "property": property, "value": value}],
                }),
                &[],
            )
            .await?;
        created_cid(&response, CREDENTIAL_OFFER)
    }

    /// The test user accepts a credential offer. The user has no user service,
    /// so the offer's own free-accept choice is the way in.
    pub(crate) async fn accept_offer(&self, offer_cid: &str) -> Result<(), String> {
        self.ledger
            .exercise(
                &[&self.user],
                CREDENTIAL_OFFER,
                offer_cid,
                "CredentialOffer_AcceptFree",
                json!({}),
                &[],
            )
            .await
            .map(|_| ())
    }
}

impl<A: CantonBridgeV1> AssetStack<A> {
    /// The asset's rules as the typed model the flows take.
    pub(crate) fn account_rules(&self) -> AccountContractRuleSet<A> {
        // The model's asset marker is private to its module, so the model
        // comes from JSON.
        serde_json::from_value(json!({
            "da_rules": self.deposit_rules,
            "wa_rules": self.withdraw_rules,
        }))
        .expect("rules")
    }

    /// The registrar's utility contracts for this asset, as the typed model
    /// the burn takes.
    pub(crate) fn token_standard_contracts(&self) -> TokenStandardContracts<A> {
        serde_json::from_value(json!({
            "burn_mint_factory": self.allocation_factory,
            "instrument_configuration": self.instrument_configuration,
        }))
        .expect("token standard contracts")
    }

    /// The asset's instrument, with the test registrar as its admin.
    pub(crate) fn instrument(&self) -> InstrumentId {
        InstrumentId {
            admin: self.registrar.clone(),
            id: A::TICKER.to_string(),
        }
    }
}

/// The argument of `UserService_OfferFreeCredential` that offers `holder` a
/// credential with the claim `hasRegistryRole = <role>`.
fn credential_offer(holder: &str, name: &str, role: &str) -> serde_json::Value {
    json!({
        "holder": holder,
        "id": format!("localnet-{}-credential", name),
        "description": format!("localnet {} credential", name),
        "claims": [{"subject": holder, "property": "hasRegistryRole", "value": role}],
    })
}

/// The active contract `contract_id` with its ledger blob, as `party` sees it.
async fn read_back(
    ledger: &Ledger,
    party: &str,
    template_id: &str,
    contract_id: &str,
) -> Result<ContractInfo, String> {
    ledger
        .active(party, template_id)
        .await?
        .iter()
        .find(|contract| contract.created_event.contract_id == contract_id)
        .map(contract_info)
        .ok_or_else(|| format!("{} {} is not active", template_id, contract_id))
}
