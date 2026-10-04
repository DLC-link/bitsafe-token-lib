//! The CBTC localnet suite: one test that runs the phases in order over one
//! fixture, because each phase uses the state the previous one left.

use serde_json::json;

use crate::{
    DamlDecimal, Network,
    credentials::{
        CredentialOffer, ListCredentialOffersParams, ListCredentialsParams, UserCredential,
        list_credential_offers, list_credentials,
    },
    flows::canton_bridge_v1::{
        CantonBridgeV1,
        credentials::{minter_credential_cids, minter_credential_offers},
        deposit::{
            CreateDepositAccountParams, ListDepositAccountsParams, create_deposit_account,
            find_deposit_account, list_deposit_accounts,
        },
        models::DepositAccount,
    },
    kits::canton::{self, ListHoldingsParams, list_holdings},
    localnet::{
        fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION},
        governance::run_action,
    },
    tokens::cbtc::{self, Cbtc},
};

const DEPOSIT_COMPLETE_ACTION: &str =
    "#cbtc-governance-actions-v1:CBTC.GovernanceActions.DepositComplete:DepositCompleteAction";

#[tokio::test]
#[ignore = "needs the localnet sandbox; see src/localnet/mod.rs"]
async fn localnet_cbtc() {
    let fixture = Fixture::new().await.expect("fixture");
    let rules = fixture
        .ledger
        .active(&fixture.registrar, GOVERNANCE_RULES)
        .await
        .expect("governance rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].created_event.contract_id, fixture.governance_rules);
    let instrument_configs = fixture
        .ledger
        .active(&fixture.registrar, INSTRUMENT_CONFIGURATION)
        .await
        .expect("instrument configurations");
    assert_eq!(instrument_configs.len(), 1);
    let rules = fixture.account_rules::<Cbtc>();
    assert_eq!(
        rules.da_rules.contract_id,
        fixture.deposit_rules.contract_id
    );
    assert!(!rules.da_rules.created_event_blob.is_empty());
    let contracts = fixture.token_standard_contracts::<Cbtc>();
    assert!(contracts.issuer_credential.is_some());
    assert_eq!(fixture.instrument().id, "CBTC");

    let minter_cids = credentials_phase(&fixture).await;
    // The burn phase uses the account.
    let _account = mint_phase(&fixture, &minter_cids).await;
}

/// The credentials phase. The registrar offers the test user the CBTC Minter
/// claim, and the user accepts it. Returns the user's Minter credential cids.
async fn credentials_phase(fixture: &Fixture) -> Vec<String> {
    let registrar = fixture.registrar.as_str();

    // Before the offer, the user holds no Minter credential and no offer.
    let credentials = user_credentials(fixture).await;
    assert!(minter_credential_cids::<Cbtc>(registrar, &credentials).is_empty());
    let offers = user_offers(fixture).await;
    assert!(minter_credential_offers::<Cbtc>(registrar, &offers).is_empty());

    let offer = fixture
        .offer_minter_credential()
        .await
        .expect("offer the minter credential");
    let offers = user_offers(fixture).await;
    let pending = minter_credential_offers::<Cbtc>(registrar, &offers);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, offer);

    fixture
        .accept_offer(&offer)
        .await
        .expect("accept the minter credential");
    let credentials = user_credentials(fixture).await;
    let minter_cids = minter_credential_cids::<Cbtc>(registrar, &credentials);
    assert_eq!(minter_cids.len(), 1);

    // The public wrapper checks the network's registrar, which is not the
    // sandbox registrar.
    assert!(cbtc::minter_credential_cids(Network::Devnet, &credentials).is_empty());
    minter_cids
}

/// The mint phase. The user opens a deposit account with its Minter
/// credential, and the registrar completes a deposit of 1.0 into it through
/// governance. Returns the deposit account as the ledger lists it after the
/// mint.
async fn mint_phase(fixture: &Fixture, minter_cids: &[String]) -> DepositAccount<Cbtc> {
    assert!(user_deposit_accounts(fixture).await.is_empty());

    let account = create_deposit_account::<Cbtc>(CreateDepositAccountParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account_rules: fixture.account_rules(),
        credential_cids: minter_cids.to_vec(),
    })
    .await
    .expect("create the deposit account");
    assert_eq!(account.owner, fixture.user);
    assert_eq!(account.registrar, fixture.registrar);
    let accounts = user_deposit_accounts(fixture).await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].contract_id, account.contract_id);
    let found = find_deposit_account::<Cbtc>(deposit_params(fixture), &account.contract_id)
        .await
        .expect("find the deposit account");
    assert_eq!(found.contract_id, account.contract_id);

    // The extra args name the instrument configuration and the registrar's
    // credentials. The mint reads both.
    let context = json!({
        "values": {
            "utility.digitalasset.com/instrument-configuration":
                {"tag": "AV_ContractId", "value": fixture.instrument_configuration.contract_id},
            "utility.digitalasset.com/issuer-credentials":
                {"tag": "AV_List", "value": fixture.issuer_credentials.iter()
                    .map(|c| json!({"tag": "AV_ContractId", "value": c.contract_id}))
                    .collect::<Vec<_>>()},
        }
    });
    let block = account.last_processed_block + 1;
    let response = run_action(
        fixture,
        DEPOSIT_COMPLETE_ACTION,
        json!({
            "governanceParty": fixture.registrar,
            "proposer": fixture.registrar,
            "description": "localnet deposit",
            "depositAccountCid": account.contract_id,
            "amount": "1.0",
            "bitcoinBlock": block.to_string(),
            "burnMintFactoryCid": fixture.allocation_factory.contract_id,
            "extraArgs": {"context": context, "meta": {"values": {}}},
            "credentialCids": minter_cids,
        }),
    )
    .await
    .expect("deposit completion");

    let holdings = list_holdings(ListHoldingsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        instrument_id: fixture.instrument(),
    })
    .await
    .expect("list the user's holdings");
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].amount, DamlDecimal::parse("1.0").unwrap());

    // The mint replaces the account. The new contract keeps the old id as
    // its stable id and records the deposit's block.
    let minted =
        canton::created_by_suffix(&response, canton::template_suffix(Cbtc::DEPOSIT_ACCOUNT))
            .expect("the mint recreates the deposit account")
            .contract_id
            .clone();
    let account_after = find_deposit_account::<Cbtc>(deposit_params(fixture), &minted)
        .await
        .expect("find the deposit account after the mint");
    assert_eq!(account_after.account_id(), account.account_id());
    assert_eq!(account_after.last_processed_block, block);
    assert_eq!(user_deposit_accounts(fixture).await.len(), 1);
    account_after
}

/// The parameters that list the test user's deposit accounts.
fn deposit_params(fixture: &Fixture) -> ListDepositAccountsParams {
    ListDepositAccountsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    }
}

/// The CBTC deposit accounts the test user holds.
async fn user_deposit_accounts(fixture: &Fixture) -> Vec<DepositAccount<Cbtc>> {
    list_deposit_accounts::<Cbtc>(deposit_params(fixture))
        .await
        .expect("list the user's deposit accounts")
}

/// The credentials the test user holds.
async fn user_credentials(fixture: &Fixture) -> Vec<UserCredential> {
    list_credentials(ListCredentialsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credentials")
}

/// The credential offers the test user holds.
async fn user_offers(fixture: &Fixture) -> Vec<CredentialOffer> {
    list_credential_offers(ListCredentialOffersParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credential offers")
}
