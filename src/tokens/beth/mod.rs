//! BETH: ETH bridged to Canton. This module is the whole BETH API.

use alloy_primitives::address;
use serde_json::{Map, Value};

use crate::{
    AssetInfo, InstrumentId, KeycloakConfig, Network, TokenClientConfig, TokenStandardVersion,
    flows::canton_bridge_v1::CantonBridgeV1, kits::evm,
};

pub use crate::kits::evm::evm_asset_bridge::EvmBridge;

/// BETH as a type. It has no methods: it carries the asset in a model's
/// type, so a BETH account cannot reach another asset's function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beth;

/// The BETH instrument id.
pub const TICKER: &str = <Beth as CantonBridgeV1>::TICKER;

/// BETH's identity for tooling that iterates over assets.
pub const INFO: AssetInfo = AssetInfo {
    ticker: TICKER,
    registrar,
    minter_claim: <Beth as CantonBridgeV1>::MINTER_CLAIM,
    dar_dirs: &["dars/dependencies", "dars/beth"],
};

/// BETH's values on one network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BethNetworkConfig {
    /// The registrar that administers BETH on the network. It becomes the
    /// instrument's admin; a wrong one raises no error and reads a zero balance.
    pub registrar: &'static str,
    /// The bridge deployment that takes the network's ETH deposits.
    pub bridge: EvmBridge,
}

const DEVNET: BethNetworkConfig = BethNetworkConfig {
    registrar: "beth-network::12207547956b2fbcc5c7b85ee3eeb13705b641e233fb17cebb4057d14e1a3f81bea6",
    bridge: EvmBridge {
        chain_id: 11_155_111,
        proxy: address!("0xaAafDA111EAf5a49359e5C15B9B58DED7313f979"),
        deploy_block: 11_680_882,
    },
};

const TESTNET: BethNetworkConfig = BethNetworkConfig {
    registrar: "beth-network::122027d91679105c721435c6f08f8d0cd684e6679f385731f5b3691cb9bf93cd126e",
    bridge: EvmBridge {
        chain_id: 11_155_111,
        proxy: address!("0x5D60b6C3eC2FD5E4227AE0008BABE95Be86dD172"),
        deploy_block: 11_724_155,
    },
};

const MAINNET: BethNetworkConfig = BethNetworkConfig {
    registrar: "beth-network::1220704c3cebc23916785557ebe79a5c7f68d034890a5b15690ea7ce050c7d463075",
    bridge: EvmBridge {
        chain_id: 1,
        proxy: address!("0xb5d425d678B10949dD0f7bB0dd0427C47451D57E"),
        deploy_block: 26_032_890,
    },
};

/// BETH's values on a network. Every BETH function that takes `Network`
/// reads them here, so the value a check compares against is the value the
/// call uses.
pub const fn config(network: Network) -> &'static BethNetworkConfig {
    match network {
        Network::Devnet => &DEVNET,
        Network::Testnet => &TESTNET,
        Network::Mainnet => &MAINNET,
    }
}

/// The registrar that administers BETH on a network.
pub const fn registrar(network: Network) -> &'static str {
    config(network).registrar
}

/// The bridge deployment that takes ETH deposits on a network.
pub const fn bridge(network: Network) -> EvmBridge {
    config(network).bridge
}

/// The BETH instrument on a network, for every Token Standard call.
pub fn instrument(network: Network) -> InstrumentId {
    InstrumentId {
        admin: registrar(network).to_string(),
        id: TICKER.to_string(),
    }
}

/// A `TokenClient` configuration bound to BETH on a network.
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

impl CantonBridgeV1 for Beth {
    const TICKER: &'static str = "BETH";
    const API_PATH: &'static str = "beth";
    const DEPOSIT_ACCOUNT: &'static str = "#beth:BETH.DepositAccount:BETHDepositAccount";
    const DEPOSIT_ACCOUNT_RULES: &'static str = "#beth:BETH.DepositAccount:BETHDepositAccountRules";
    const WITHDRAW_ACCOUNT: &'static str = "#beth:BETH.WithdrawAccount:BETHWithdrawAccount";
    const WITHDRAW_ACCOUNT_RULES: &'static str =
        "#beth:BETH.WithdrawAccount:BETHWithdrawAccountRules";
    const WITHDRAW_REQUEST: &'static str = "#beth:BETH.WithdrawRequest:BETHWithdrawRequest";
    const CREATE_DEPOSIT_ACCOUNT_CHOICE: &'static str =
        "BETHDepositAccountRules_CreateDepositAccount";
    const CREATE_WITHDRAW_ACCOUNT_CHOICE: &'static str =
        "BETHWithdrawAccountRules_CreateWithdrawAccount";
    const WITHDRAW_CHOICE: &'static str = "BETHWithdrawAccount_Withdraw";
    const LAST_PROCESSED_BLOCK_FIELD: &'static str = "lastProcessedEthereumBlock";
    const DESTINATION_ADDRESS_FIELD: &'static str = "destinationEthAddress";
    const MINTER_CLAIM: (&'static str, &'static str) = ("hasBETHRole", "Minter");
    const WITHDRAW_REASON: &'static str = "BETH withdrawal";

    /// Stricter than the Daml check, which tests the shape only: an address
    /// with an uppercase hex letter must also match its EIP-55 checksum, and
    /// the zero address is refused, because nobody can spend ETH paid to it.
    fn validate_destination(address: &str) -> Result<(), String> {
        evm::validate_address(address)?;
        if address[2..].bytes().all(|byte| byte == b'0') {
            return Err("destination address is the zero address".to_string());
        }
        Ok(())
    }

    type WithdrawRequestDetails = ();

    /// A live `BETHWithdrawRequest` has no transaction id. `ethTxId` is an
    /// argument of the completion choice, which archives the request.
    fn parse_withdraw_request_details(
        _args: &Map<String, Value>,
    ) -> Result<Self::WithdrawRequestDetails, String> {
        Ok(())
    }
}

crate::flows::canton_bridge_v1::bind::bind_canton_bridge_v1!(Beth);

pub use family::{minter_credential_cids, minter_credential_offers};

/// Minting BETH: deposit accounts and the `depositETH` call.
///
/// The alloy types re-exported here are the ones the BETH signatures use.
/// `Address` prints as its EIP-55 form, `Bytes` as `0x` hex and `U256` as
/// a decimal number, so a caller without alloy can print every value.
pub mod mint {
    pub use super::family::mint::*;
    pub use alloy_primitives::{Address, B256, Bytes, U256};
}

/// Redeeming BETH: withdraw accounts, the burn and the payout records.
pub mod redeem {
    pub use super::family::redeem::*;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        DamlDecimal,
        flows::canton_bridge_v1::models::{DepositAccount, WithdrawRequest},
        test_fixtures::active_contract_of,
    };

    const DA: &str = "99a3:BETH.DepositAccount:BETHDepositAccount";
    const WA: &str = "99a3:BETH.WithdrawAccount:BETHWithdrawAccount";
    const WR: &str = "99a3:BETH.WithdrawRequest:BETHWithdrawRequest";
    const CHECKSUMMED: &str = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";

    /// Each literal spelled out, because comparing a function against the
    /// constant it returns cannot fail.
    #[test]
    fn each_network_has_its_own_registrar() {
        assert_eq!(
            registrar(Network::Devnet),
            "beth-network::12207547956b2fbcc5c7b85ee3eeb13705b641e233fb17cebb4057d14e1a3f81bea6"
        );
        assert_eq!(
            registrar(Network::Testnet),
            "beth-network::122027d91679105c721435c6f08f8d0cd684e6679f385731f5b3691cb9bf93cd126e"
        );
        assert_eq!(
            registrar(Network::Mainnet),
            "beth-network::1220704c3cebc23916785557ebe79a5c7f68d034890a5b15690ea7ce050c7d463075"
        );
    }

    #[test]
    fn each_network_has_its_own_bridge() {
        let devnet = bridge(Network::Devnet);
        assert_eq!(devnet.chain_id, 11_155_111);
        assert_eq!(
            devnet.proxy.to_string(),
            "0xaAafDA111EAf5a49359e5C15B9B58DED7313f979"
        );
        assert_eq!(devnet.deploy_block, 11_680_882);
        let testnet = bridge(Network::Testnet);
        assert_eq!(testnet.chain_id, 11_155_111);
        assert_eq!(
            testnet.proxy.to_string(),
            "0x5D60b6C3eC2FD5E4227AE0008BABE95Be86dD172"
        );
        assert_eq!(testnet.deploy_block, 11_724_155);
        let mainnet = bridge(Network::Mainnet);
        assert_eq!(mainnet.chain_id, 1);
        assert_eq!(
            mainnet.proxy.to_string(),
            "0xb5d425d678B10949dD0f7bB0dd0427C47451D57E"
        );
        assert_eq!(mainnet.deploy_block, 26_032_890);
    }

    #[test]
    fn instrument_is_the_registrar_and_the_ticker() {
        let instrument = instrument(Network::Testnet);
        assert_eq!(instrument.admin, registrar(Network::Testnet));
        assert_eq!(instrument.id, "BETH");
    }

    #[test]
    fn info_maps_each_registrar_back_to_its_network() {
        for network in Network::ALL {
            assert_eq!(INFO.network_of(registrar(network)), Some(network));
        }
        assert_eq!(INFO.ticker, "BETH");
        assert_eq!(INFO.minter_claim, ("hasBETHRole", "Minter"));
        assert_eq!(INFO.dar_dirs, &["dars/dependencies", "dars/beth"]);
    }

    #[test]
    fn a_destination_must_be_a_checksummed_or_lowercase_eth_address() {
        assert_eq!(Beth::validate_destination(CHECKSUMMED), Ok(()));
        assert_eq!(
            Beth::validate_destination(&CHECKSUMMED.to_lowercase()),
            Ok(())
        );
        assert_eq!(
            Beth::validate_destination("0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed").unwrap_err(),
            "address 0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed does not match its EIP-55 checksum"
        );
        assert_eq!(
            Beth::validate_destination("bcrt1qexample00000").unwrap_err(),
            "address \"bcrt1qexample00000\" is not 0x and 40 hex digits"
        );
    }

    #[test]
    fn the_zero_address_is_not_a_destination() {
        assert_eq!(
            Beth::validate_destination("0x0000000000000000000000000000000000000000").unwrap_err(),
            "destination address is the zero address"
        );
        assert_eq!(
            Beth::validate_destination("0x0000000000000000000000000000000000000001"),
            Ok(())
        );
    }

    fn deposit_args(id: Value) -> Value {
        json!({
            "id": id, "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "instrument": {"admin": registrar(Network::Devnet), "id": "BETH"},
            "lastProcessedEthereumBlock": "11680900",
            "limits": {"minAmount": "0.01", "maxAmount": "100"},
        })
    }

    #[test]
    fn a_beth_deposit_account_parses_its_ethereum_block() {
        let account = DepositAccount::<Beth>::from_active_contract(&active_contract_of(
            DA,
            "00da",
            deposit_args(json!("00orig")),
        ))
        .unwrap();
        assert_eq!(account.last_processed_block, 11_680_900);
        assert_eq!(account.account_id(), "00orig");
        assert_eq!(account.registrar, registrar(Network::Devnet));
    }

    #[test]
    fn a_beth_deposit_account_without_its_block_field_fails_by_name() {
        let mut args = deposit_args(json!(null));
        args.as_object_mut()
            .unwrap()
            .remove("lastProcessedEthereumBlock");
        let error =
            DepositAccount::<Beth>::from_active_contract(&active_contract_of(DA, "00da", args))
                .unwrap_err();
        assert_eq!(
            error,
            "Missing or invalid 'lastProcessedEthereumBlock' field"
        );
    }

    #[test]
    fn a_beth_withdraw_account_parses_its_eth_destination() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "id": null, "destinationEthAddress": CHECKSUMMED, "pendingBalance": "0", "limits": null,
        });
        let account =
            redeem::WithdrawAccount::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert_eq!(account.destination_address, CHECKSUMMED);
        assert_eq!(account.pending_balance, DamlDecimal::ZERO);
    }

    #[test]
    fn a_beth_withdraw_request_parses_with_or_without_its_source_account() {
        let with_source = json!({
            "owner": "alice", "registrar": registrar(Network::Devnet), "amount": "0.25",
            "destinationEthAddress": CHECKSUMMED, "sourceAccountId": "00wa",
        });
        let request = WithdrawRequest::<Beth>::from_active_contract(&active_contract_of(
            WR,
            "00wr",
            with_source,
        ))
        .unwrap();
        assert_eq!(request.amount, DamlDecimal::parse("0.25").unwrap());
        assert_eq!(request.destination_address, CHECKSUMMED);
        assert_eq!(request.source_account_id.as_deref(), Some("00wa"));

        let without_source = json!({
            "owner": "alice", "registrar": registrar(Network::Devnet), "amount": "0.25",
            "destinationEthAddress": CHECKSUMMED, "sourceAccountId": null,
        });
        let request = WithdrawRequest::<Beth>::from_active_contract(&active_contract_of(
            WR,
            "00wr",
            without_source,
        ))
        .unwrap();
        assert_eq!(request.source_account_id, None);
    }

    #[test]
    fn minter_credential_cids_keep_the_beth_claim_of_the_network_registrar() {
        use token::credentials::{Claim, UserCredential};
        let credential = |cid: &str, property: &str| UserCredential {
            contract_id: cid.to_string(),
            template_id: String::new(),
            issuer: registrar(Network::Testnet).to_string(),
            holder: "alice".to_string(),
            id: String::new(),
            description: String::new(),
            claims: vec![Claim {
                subject: "alice".to_string(),
                property: property.to_string(),
                value: "Minter".to_string(),
            }],
        };
        let credentials = [
            credential("00beth", "hasBETHRole"),
            credential("00cbtc", "hasCBTCRole"),
        ];
        assert_eq!(
            minter_credential_cids(Network::Testnet, &credentials),
            vec!["00beth".to_string()]
        );
        assert!(minter_credential_cids(Network::Mainnet, &credentials).is_empty());
    }

    #[test]
    fn minter_credential_offers_keep_the_beth_claim_of_the_network_registrar() {
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
                property: "hasBETHRole".to_string(),
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

    /// The destination check runs before any request. The test has no
    /// network; an HTTP call would fail with a different message.
    #[tokio::test]
    async fn create_withdraw_account_refuses_a_bad_checksum_before_any_request() {
        let account_rules: redeem::AccountContractRuleSet = serde_json::from_value(json!({
            "da_rules": {"contract_id": "00dr", "template_id": "t", "created_event_blob": "b"},
            "wa_rules": {"contract_id": "00wr", "template_id": "t", "created_event_blob": "b"},
        }))
        .unwrap();
        let params = redeem::CreateWithdrawAccountParams {
            ledger_host: "not-a-host".to_string(),
            party: "alice".to_string(),
            access_token: "token".to_string(),
            account_rules,
            destination_address: "0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
            credential_cids: vec!["00cred".to_string()],
        };
        let error = match redeem::create_withdraw_account(params).await {
            Ok(_) => panic!("a bad checksum must be refused"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "address 0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed does not match its EIP-55 checksum"
        );
    }

    /// The registrar check runs before any request. The test has no network;
    /// an HTTP call would fail with a different message.
    #[tokio::test]
    async fn submit_withdraw_refuses_an_account_from_another_network() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "destinationEthAddress": CHECKSUMMED, "pendingBalance": "0", "limits": null,
        });
        let account =
            redeem::WithdrawAccount::from_active_contract(&active_contract_of(WA, "00wa", args))
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
