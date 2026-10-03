//! CBTC: Bitcoin bridged to Canton. This module is the whole CBTC API.

use serde_json::{Map, Value};

use crate::{
    AssetInfo, InstrumentId, KeycloakConfig, Network, TokenClientConfig, TokenStandardVersion,
    flows::canton_bridge_v1::CantonBridgeV1, kits::canton::required_str,
};

/// CBTC as a type. It has no methods: it carries the asset in a model's
/// type, so a CBTC account cannot reach another asset's function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cbtc;

/// The CBTC instrument id.
pub const TICKER: &str = <Cbtc as CantonBridgeV1>::TICKER;

/// CBTC's identity for tooling that iterates over assets.
pub const INFO: AssetInfo = AssetInfo {
    ticker: TICKER,
    registrar,
    minter_claim: <Cbtc as CantonBridgeV1>::MINTER_CLAIM,
    dar_dirs: &["dars/dependencies", "dars/cbtc"],
};

/// The registrar that administers CBTC on a network. It becomes the
/// instrument's admin; a wrong one raises no error and reads a zero balance.
pub const fn registrar(network: Network) -> &'static str {
    match network {
        Network::Devnet => common::consts::DEVNET_DECENTRALIZED_PARTY_ID,
        Network::Testnet => common::consts::TESTNET_DECENTRALIZED_PARTY_ID,
        Network::Mainnet => common::consts::MAINNET_DECENTRALIZED_PARTY_ID,
    }
}

/// The CBTC instrument on a network, for every Token Standard call.
pub fn instrument(network: Network) -> InstrumentId {
    InstrumentId {
        admin: registrar(network).to_string(),
        id: TICKER.to_string(),
    }
}

/// A `TokenClient` configuration bound to CBTC on a network.
pub fn client_config(
    network: Network,
    ledger_host: String,
    party: String,
    keycloak: KeycloakConfig,
    version: TokenStandardVersion,
) -> TokenClientConfig {
    TokenClientConfig {
        ledger_host,
        registry_url: network.registry_url().to_string(),
        instrument: instrument(network),
        party,
        keycloak,
        version,
    }
}

impl redeem::WithdrawRequest {
    /// The Bitcoin transaction id of the payout, which the registrar sets
    /// when it creates the request.
    pub fn btc_tx_id(&self) -> &str {
        &self.details.btc_tx_id
    }
}

impl CantonBridgeV1 for Cbtc {
    const TICKER: &'static str = "CBTC";
    const API_PATH: &'static str = "cbtc";
    const DEPOSIT_ACCOUNT: &'static str = "#cbtc:CBTC.DepositAccount:CBTCDepositAccount";
    const DEPOSIT_ACCOUNT_RULES: &'static str = "#cbtc:CBTC.DepositAccount:CBTCDepositAccountRules";
    const WITHDRAW_ACCOUNT: &'static str = "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccount";
    const WITHDRAW_ACCOUNT_RULES: &'static str =
        "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccountRules";
    const WITHDRAW_REQUEST: &'static str = "#cbtc:CBTC.WithdrawRequest:CBTCWithdrawRequest";
    const CREATE_DEPOSIT_ACCOUNT_CHOICE: &'static str =
        "CBTCDepositAccountRules_CreateDepositAccount";
    const CREATE_WITHDRAW_ACCOUNT_CHOICE: &'static str =
        "CBTCWithdrawAccountRules_CreateWithdrawAccount";
    const WITHDRAW_CHOICE: &'static str = "CBTCWithdrawAccount_Withdraw";
    const LAST_PROCESSED_BLOCK_FIELD: &'static str = "lastProcessedBitcoinBlock";
    const DESTINATION_ADDRESS_FIELD: &'static str = "destinationBtcAddress";
    const MINTER_CLAIM: (&'static str, &'static str) = ("hasCBTCRole", "Minter");
    const WITHDRAW_REASON: &'static str = "CBTC withdrawal";

    /// Mirrors the Daml check: not empty, 14 to 74 characters. A strict
    /// Bitcoin address parse is on the follow-up list.
    fn validate_destination(address: &str) -> Result<(), String> {
        let length = address.chars().count();
        if length == 0 {
            return Err("destination address is empty".to_string());
        }
        if !(14..=74).contains(&length) {
            return Err(format!(
                "destination address has {length} characters, expected 14 to 74"
            ));
        }
        Ok(())
    }

    type WithdrawRequestDetails = redeem::WithdrawRequestDetails;

    /// `CBTCWithdrawRequest` declares `btcTxId: Text`, so a request without
    /// it fails, as it did in `cbtc-lib`.
    fn parse_withdraw_request_details(
        args: &Map<String, Value>,
    ) -> Result<Self::WithdrawRequestDetails, String> {
        Ok(redeem::WithdrawRequestDetails {
            btc_tx_id: required_str(args, "btcTxId")?,
        })
    }
}

crate::flows::canton_bridge_v1::bind::bind_canton_bridge_v1!(Cbtc);

pub use family::{minter_credential_cids, minter_credential_offers};

/// Minting CBTC: deposit accounts and their Bitcoin addresses.
pub mod mint {
    use serde::Deserialize;

    pub use super::family::mint::*;
    use super::registrar;
    use crate::{Network, flows::canton_bridge_v1::check_registrar, kits::bitsafe_api};

    #[derive(Deserialize)]
    struct BitcoinAddressResponse {
        bitcoin_address: String,
    }

    /// A deposit account with its Bitcoin address.
    #[derive(Debug, Clone)]
    pub struct DepositAccountStatus {
        pub contract_id: String,
        pub owner: String,
        pub operator: String,
        pub registrar: String,
        pub bitcoin_address: String,
        pub last_processed_block: i64,
        pub limits: Option<Limits>,
    }

    /// Asks the BitSafe API for the Bitcoin address the attestors derive for
    /// this account.
    ///
    /// # Errors
    ///
    /// Fails before any request when the account belongs to another network.
    /// The attestors derive an address for any id, so BTC sent to an address
    /// for the wrong network is never credited.
    pub async fn get_bitcoin_address(
        network: Network,
        account: &DepositAccount,
    ) -> Result<String, String> {
        check_registrar(&account.registrar, registrar(network))?;
        let response: BitcoinAddressResponse = bitsafe_api::get_json(
            network.bitsafe_api_url(),
            &format!("cbtc/v1/bitcoin-address/{}", account.account_id()),
        )
        .await?;
        Ok(response.bitcoin_address)
    }

    /// Finds a deposit account by contract id and adds its Bitcoin address.
    pub async fn get_deposit_account_status(
        network: Network,
        params: ListDepositAccountsParams,
        contract_id: &str,
    ) -> Result<DepositAccountStatus, String> {
        let account = find_deposit_account(params, contract_id).await?;
        let bitcoin_address = get_bitcoin_address(network, &account).await?;
        Ok(DepositAccountStatus {
            contract_id: account.contract_id,
            owner: account.owner,
            operator: account.operator,
            registrar: account.registrar,
            bitcoin_address,
            last_processed_block: account.last_processed_block,
            limits: account.limits,
        })
    }
}

/// Redeeming CBTC: withdraw accounts, the burn and the payout records.
pub mod redeem {
    pub use super::family::redeem::*;

    /// The CBTC-only fields of a withdraw request.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WithdrawRequestDetails {
        /// The Bitcoin transaction id of the payout.
        pub btc_tx_id: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each literal spelled out, because comparing a function against the
    /// constant it returns cannot fail.
    #[test]
    fn each_network_has_its_own_registrar() {
        assert_eq!(
            registrar(Network::Devnet),
            "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff"
        );
        assert_eq!(
            registrar(Network::Testnet),
            "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f"
        );
        assert_eq!(
            registrar(Network::Mainnet),
            "cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262"
        );
    }

    #[test]
    fn instrument_is_the_registrar_and_the_ticker() {
        let instrument = instrument(Network::Testnet);
        assert_eq!(instrument.admin, registrar(Network::Testnet));
        assert_eq!(instrument.id, "CBTC");
    }

    #[test]
    fn info_maps_each_registrar_back_to_its_network() {
        for network in Network::ALL {
            assert_eq!(INFO.network_of(registrar(network)), Some(network));
        }
        assert_eq!(INFO.ticker, "CBTC");
        assert_eq!(INFO.minter_claim, ("hasCBTCRole", "Minter"));
    }

    #[test]
    fn a_destination_of_14_to_74_characters_passes() {
        assert!(Cbtc::validate_destination(&"b".repeat(14)).is_ok());
        assert!(Cbtc::validate_destination(&"b".repeat(74)).is_ok());
    }

    #[test]
    fn a_destination_outside_14_to_74_characters_fails_with_its_length() {
        assert_eq!(
            Cbtc::validate_destination("").unwrap_err(),
            "destination address is empty"
        );
        assert_eq!(
            Cbtc::validate_destination(&"b".repeat(13)).unwrap_err(),
            "destination address has 13 characters, expected 14 to 74"
        );
        assert_eq!(
            Cbtc::validate_destination(&"b".repeat(75)).unwrap_err(),
            "destination address has 75 characters, expected 14 to 74"
        );
    }

    use serde_json::json;

    use crate::{DamlDecimal, test_fixtures::active_contract_of};

    fn devnet_account() -> mint::DepositAccount {
        let args = json!({
            "id": null, "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "lastProcessedBitcoinBlock": "1", "limits": null,
        });
        crate::flows::canton_bridge_v1::models::DepositAccount::from_active_contract(
            &active_contract_of("f240:CBTC.DepositAccount:CBTCDepositAccount", "00da", args),
        )
        .unwrap()
    }

    /// The attestor derives an address for any id, so this must fail before
    /// any request. The test has no network; an HTTP call would fail with a
    /// different message.
    #[tokio::test]
    async fn get_bitcoin_address_refuses_an_account_from_another_network() {
        let error = mint::get_bitcoin_address(Network::Mainnet, &devnet_account())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            format!(
                "network mismatch: account registrar {}, expected {}",
                registrar(Network::Devnet),
                registrar(Network::Mainnet)
            )
        );
    }

    #[test]
    fn minter_credential_cids_use_the_network_registrar() {
        use token::credentials::{Claim, UserCredential};
        let credential = UserCredential {
            contract_id: "00c".to_string(),
            template_id: String::new(),
            issuer: registrar(Network::Testnet).to_string(),
            holder: "alice".to_string(),
            id: String::new(),
            description: String::new(),
            claims: vec![Claim {
                subject: "alice".to_string(),
                property: "hasCBTCRole".to_string(),
                value: "Minter".to_string(),
            }],
        };
        assert_eq!(
            minter_credential_cids(Network::Testnet, std::slice::from_ref(&credential)),
            vec!["00c".to_string()]
        );
        assert!(minter_credential_cids(Network::Mainnet, &[credential]).is_empty());
    }

    #[test]
    fn minter_credential_offers_use_the_network_registrar() {
        use token::credentials::{Claim, CredentialOffer};
        let offer = CredentialOffer {
            contract_id: "00o".to_string(),
            template_id: String::new(),
            created_event_blob: String::new(),
            issuer: registrar(Network::Testnet).to_string(),
            holder: "alice".to_string(),
            id: String::new(),
            description: String::new(),
            claims: vec![Claim {
                subject: "alice".to_string(),
                property: "hasCBTCRole".to_string(),
                value: "Minter".to_string(),
            }],
        };
        let offers = [offer];
        let kept: Vec<&str> = minter_credential_offers(Network::Testnet, &offers)
            .into_iter()
            .map(|offer| offer.contract_id.as_str())
            .collect();
        assert_eq!(kept, vec!["00o"]);
        assert!(minter_credential_offers(Network::Mainnet, &offers).is_empty());
    }

    /// The registrar check runs before any request. The test has no network;
    /// an HTTP call would fail with a different message.
    #[tokio::test]
    async fn submit_withdraw_refuses_an_account_from_another_network() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "destinationBtcAddress": "bcrt1qexample00000", "pendingBalance": "0", "limits": null,
        });
        let account = redeem::WithdrawAccount::from_active_contract(&active_contract_of(
            "f240:CBTC.WithdrawAccount:CBTCWithdrawAccount",
            "00wa",
            args,
        ))
        .unwrap();
        let holdings = [token::holding::Holding {
            contract_id: "00h".to_string(),
            amount: DamlDecimal::parse("1").unwrap(),
            instrument_id: instrument(Network::Mainnet),
            owner: "alice".to_string(),
            account_label: String::new(),
        }];
        let params = redeem::SubmitWithdrawParams {
            ledger_host: "not-a-host".to_string(),
            party: "alice".to_string(),
            access_token: "token".to_string(),
            account: &account,
            amount: DamlDecimal::parse("0.5").unwrap(),
            holdings: &holdings,
            credential_cids: vec!["00cred".to_string()],
        };
        let error = match redeem::submit_withdraw(Network::Mainnet, params).await {
            Ok(_) => panic!("an account from another network must be refused"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            format!(
                "network mismatch: account registrar {}, expected {}",
                registrar(Network::Devnet),
                registrar(Network::Mainnet)
            )
        );
    }
}
