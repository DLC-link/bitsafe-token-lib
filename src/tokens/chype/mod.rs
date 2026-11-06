//! CHYPE: HYPE bridged from HyperEVM to Canton. This module is the whole
//! CHYPE API.

use alloy_primitives::{Address, address};
use serde_json::{Map, Value};

use crate::{
    AssetInfo, InstrumentId, KeycloakConfig, Network, TokenClientConfig, TokenStandardVersion,
    flows::canton_bridge_v1::CantonBridgeV1, kits::evm,
};

pub use crate::kits::evm::evm_asset_bridge::EvmBridge;

/// CHYPE as a type. It has no methods: it carries the asset in a model's
/// type, so a CHYPE account cannot reach another asset's function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chype;

/// The CHYPE instrument id.
pub const TICKER: &str = <Chype as CantonBridgeV1>::TICKER;

/// CHYPE's identity for tooling that iterates over assets.
pub const INFO: AssetInfo = AssetInfo {
    ticker: TICKER,
    registrar,
    minter_claim: <Chype as CantonBridgeV1>::MINTER_CLAIM,
    dar_dirs: &["dars/dependencies", "dars/chype"],
};

/// CHYPE's values on one network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChypeNetworkConfig {
    /// The registrar that administers CHYPE on the network. It becomes the
    /// instrument's admin; a wrong one raises no error and reads a zero balance.
    pub registrar: &'static str,
    /// The bridge deployment that takes the network's HYPE deposits.
    pub bridge: EvmBridge,
}

// Devnet and testnet both use HyperEVM testnet, chain 998, with one bridge
// each. Mainnet uses HyperEVM, chain 999.
const DEVNET: ChypeNetworkConfig = ChypeNetworkConfig {
    registrar: "chype-network::1220806e3c909f4a07c1131f42163fbceb3fbd1bbcbb90e8fe5a266d629ed49d2c52",
    bridge: EvmBridge {
        chain_id: 998,
        proxy: address!("0xfBC38458ffb91D6Fd731151D46345E89ffc42358"),
        deploy_block: 66_360_856,
    },
};

const TESTNET: ChypeNetworkConfig = ChypeNetworkConfig {
    registrar: "chype-network::12204c05b237be2bfafe7b46f6934858adc1561e9ad1a6996f4d1e1d24612e3b1db0",
    bridge: EvmBridge {
        chain_id: 998,
        proxy: address!("0x1E5439257A314d11e6b0779297Ca92887Ed56087"),
        deploy_block: 67_938_405,
    },
};

const MAINNET: ChypeNetworkConfig = ChypeNetworkConfig {
    registrar: "chype-network::1220cc3b5ad1ada1b7748a17414bf1473e2de0cf9dc7bc820be1cc441b78c0b15956",
    bridge: EvmBridge {
        chain_id: 999,
        proxy: address!("0x2940FcBb3C32901Df405da0E96fd97D1E2a53f34"),
        deploy_block: 49_937_745,
    },
};

/// CHYPE's values on a network. Every CHYPE function that takes `Network`
/// reads them here, so the value a check compares against is the value the
/// call uses.
pub const fn config(network: Network) -> &'static ChypeNetworkConfig {
    match network {
        Network::Devnet => &DEVNET,
        Network::Testnet => &TESTNET,
        Network::Mainnet => &MAINNET,
    }
}

/// The registrar that administers CHYPE on a network.
pub const fn registrar(network: Network) -> &'static str {
    config(network).registrar
}

/// The bridge deployment that takes HYPE deposits on a network.
pub const fn bridge(network: Network) -> EvmBridge {
    config(network).bridge
}

/// The CHYPE instrument on a network, for every Token Standard call.
pub fn instrument(network: Network) -> InstrumentId {
    InstrumentId {
        admin: registrar(network).to_string(),
        id: TICKER.to_string(),
    }
}

/// A `TokenClient` configuration bound to CHYPE on a network.
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

impl CantonBridgeV1 for Chype {
    const TICKER: &'static str = "CHYPE";
    const API_PATH: &'static str = "chype";
    const DEPOSIT_ACCOUNT: &'static str = "#chype:CHYPE.DepositAccount:CHYPEDepositAccount";
    const DEPOSIT_ACCOUNT_RULES: &'static str =
        "#chype:CHYPE.DepositAccount:CHYPEDepositAccountRules";
    const WITHDRAW_ACCOUNT: &'static str = "#chype:CHYPE.WithdrawAccount:CHYPEWithdrawAccount";
    const WITHDRAW_ACCOUNT_RULES: &'static str =
        "#chype:CHYPE.WithdrawAccount:CHYPEWithdrawAccountRules";
    const WITHDRAW_REQUEST: &'static str = "#chype:CHYPE.WithdrawRequest:CHYPEWithdrawRequest";
    const CREATE_DEPOSIT_ACCOUNT_CHOICE: &'static str =
        "CHYPEDepositAccountRules_CreateDepositAccount";
    const CREATE_WITHDRAW_ACCOUNT_CHOICE: &'static str =
        "CHYPEWithdrawAccountRules_CreateWithdrawAccount";
    const WITHDRAW_CHOICE: &'static str = "CHYPEWithdrawAccount_Withdraw";
    const LAST_PROCESSED_BLOCK_FIELD: &'static str = "lastProcessedBlock";
    const DESTINATION_ADDRESS_FIELD: &'static str = "destinationAddress";
    const MINTER_CLAIM: (&'static str, &'static str) = ("hasCHYPERole", "Minter");
    const WITHDRAW_REASON: &'static str = "CHYPE withdrawal";

    /// Stricter than the Daml check, which tests the shape only: an address
    /// with an uppercase hex letter must also match its EIP-55 checksum. The
    /// zero address and the HyperEVM system addresses are refused, because
    /// the attestors never pay HYPE to them, and the burn cannot be undone.
    fn validate_destination(address: &str) -> Result<(), String> {
        evm::validate_address(address)?;
        let parsed: Address = address
            .parse()
            .map_err(|_| format!("address {address:?} is not 0x and 40 hex digits"))?;
        if parsed.is_zero() {
            return Err("destination address is the zero address".to_string());
        }
        match hyperevm_system_address(parsed) {
            Some(reason) => Err(format!("destination address {address} is {reason}")),
            None => Ok(()),
        }
    }

    type WithdrawRequestDetails = ();

    /// A live `CHYPEWithdrawRequest` has no transaction id. `txId` is an
    /// argument of the completion choice, which archives the request.
    fn parse_withdraw_request_details(
        _args: &Map<String, Value>,
    ) -> Result<Self::WithdrawRequestDetails, String> {
        Ok(())
    }
}

/// The kind of HyperEVM system address that `destination` is, if any. The
/// attestors refuse a payout to each of them with the same rule.
///
/// HYPE paid to `0x2222…2222` leaves the EVM and credits the bridge's own
/// HyperCore account, which nothing can spend. `0x3333…3333` is the
/// CoreWriter. Every address below `0x…10000` is a precompile slot, present
/// or future. `0x20` followed by zeros and a token index is the system
/// address of a HyperCore spot token.
fn hyperevm_system_address(destination: Address) -> Option<&'static str> {
    const HYPE_SYSTEM: Address = address!("0x2222222222222222222222222222222222222222");
    const CORE_WRITER: Address = address!("0x3333333333333333333333333333333333333333");
    const RESERVED_LOW_END: Address = address!("0x000000000000000000000000000000000000ffff");
    if destination == HYPE_SYSTEM {
        return Some("the HyperEVM HYPE system address");
    }
    if destination == CORE_WRITER {
        return Some("the HyperEVM CoreWriter system contract");
    }
    if destination <= RESERVED_LOW_END {
        return Some("a reserved HyperEVM address below 0x10000");
    }
    let bytes = destination.as_slice();
    if bytes[0] == 0x20 && bytes[1..16].iter().all(|byte| *byte == 0) {
        return Some("a HyperEVM token system address");
    }
    None
}

crate::flows::canton_bridge_v1::bind::bind_canton_bridge_v1!(Chype);

pub use family::{minter_credential_cids, minter_credential_offers};

/// Minting CHYPE: deposit accounts and the `depositETH` call.
///
/// The bridge takes HYPE, HyperEVM's native coin, through the same
/// `depositETH` function as the BETH bridge. HYPE has 18 decimals, so its
/// smallest unit is called wei here too.
///
/// The alloy types re-exported here are the ones the CHYPE signatures use.
/// `Address` prints as its EIP-55 form, `Bytes` as `0x` hex and `U256` as
/// a decimal number, so a caller without alloy can print every value.
pub mod mint {
    use super::{ChypeNetworkConfig, config};
    use crate::{Network, flows::canton_bridge_v1::evm_mint};

    pub use super::family::mint::*;
    pub use crate::flows::canton_bridge_v1::evm_mint::{DepositCall, DepositTarget};
    pub use crate::kits::evm::evm_asset_bridge::{
        DEPOSIT_EVENT_TOPIC, DEPOSIT_UNIT_WEI, Deposit, daml_decimal_to_wei, deposit_id_topic,
        wei_to_daml_decimal,
    };
    pub use alloy_primitives::{Address, B256, Bytes, U256};

    /// Builds the `depositETH(bytes)` transaction that mints CHYPE to the
    /// account. The function does no I/O, so a front end can call it on
    /// every keystroke. It does not read the bridge's live
    /// `depositLimits()` or `paused()`; check both with your provider.
    ///
    /// `amount_wei` is in wei, as a `U256`: 1 HYPE is 10^18 wei, and the
    /// amount must be a whole multiple of [`DEPOSIT_UNIT_WEI`]. The burn
    /// takes the other unit: `redeem::submit_withdraw` reads its amount as
    /// a [`DamlDecimal`](crate::DamlDecimal) in HYPE, such as `0.25`.
    /// [`wei_to_daml_decimal`] and [`daml_decimal_to_wei`] convert between
    /// the two.
    ///
    /// ```
    /// use bitsafe_token::{Network, tokens::chype::mint::{self, DepositAccount, DepositCall, U256}};
    ///
    /// fn call(account: &DepositAccount) -> Result<DepositCall, String> {
    ///     // 0.25 HYPE
    ///     mint::deposit_call(Network::Devnet, account, U256::from(250_000_000_000_000_000_u64))
    /// }
    /// ```
    ///
    /// A BETH account does not compile, because the bridge would accept its
    /// id and lock the HYPE:
    ///
    /// ```compile_fail
    /// use bitsafe_token::{Network, tokens::{beth, chype::mint::{self, DepositCall, U256}}};
    ///
    /// fn call(account: &beth::mint::DepositAccount) -> Result<DepositCall, String> {
    ///     mint::deposit_call(Network::Devnet, account, U256::from(250_000_000_000_000_000_u64))
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Fails, in this order, when the account belongs to another network,
    /// when its id is empty, when the amount is zero, when the amount is
    /// not a whole multiple of 100000000 wei, when the amount is too large
    /// for a Daml decimal, and when the amount is outside the account's
    /// limits.
    pub fn deposit_call(
        network: Network,
        account: &DepositAccount,
        amount_wei: U256,
    ) -> Result<DepositCall, String> {
        deposit_call_for(config(network), account, amount_wei)
    }

    /// The body of `deposit_call`, for a registrar outside the three
    /// networks. The localnet suite calls it with the sandbox registrar.
    pub(crate) fn deposit_call_for(
        network_config: &ChypeNetworkConfig,
        account: &DepositAccount,
        amount_wei: U256,
    ) -> Result<DepositCall, String> {
        evm_mint::deposit_call(
            network_config.registrar,
            network_config.bridge,
            account,
            amount_wei,
        )
    }

    /// The chain, the proxy and the deposit id of the account, for a front
    /// end that builds the transaction itself.
    ///
    /// # Errors
    ///
    /// Fails when the account belongs to another network, and when its id
    /// is empty.
    pub fn deposit_target(
        network: Network,
        account: &DepositAccount,
    ) -> Result<DepositTarget, String> {
        let network_config = config(network);
        evm_mint::deposit_target(network_config.registrar, network_config.bridge, account)
    }
}

/// Redeeming CHYPE: withdraw accounts, the burn and the payout records.
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

    const DA: &str = "601d:CHYPE.DepositAccount:CHYPEDepositAccount";
    const WA: &str = "601d:CHYPE.WithdrawAccount:CHYPEWithdrawAccount";
    const WR: &str = "601d:CHYPE.WithdrawRequest:CHYPEWithdrawRequest";
    const CHECKSUMMED: &str = "0x90F79bf6EB2c4f870365E785982E1f101E93b906";

    /// Each literal spelled out, because comparing a function against the
    /// constant it returns cannot fail.
    #[test]
    fn each_network_has_its_own_registrar() {
        assert_eq!(
            registrar(Network::Devnet),
            "chype-network::1220806e3c909f4a07c1131f42163fbceb3fbd1bbcbb90e8fe5a266d629ed49d2c52"
        );
        assert_eq!(
            registrar(Network::Testnet),
            "chype-network::12204c05b237be2bfafe7b46f6934858adc1561e9ad1a6996f4d1e1d24612e3b1db0"
        );
        assert_eq!(
            registrar(Network::Mainnet),
            "chype-network::1220cc3b5ad1ada1b7748a17414bf1473e2de0cf9dc7bc820be1cc441b78c0b15956"
        );
    }

    #[test]
    fn each_network_has_its_own_bridge() {
        let devnet = bridge(Network::Devnet);
        assert_eq!(devnet.chain_id, 998);
        assert_eq!(
            devnet.proxy.to_string(),
            "0xfBC38458ffb91D6Fd731151D46345E89ffc42358"
        );
        assert_eq!(devnet.deploy_block, 66_360_856);
        let testnet = bridge(Network::Testnet);
        assert_eq!(testnet.chain_id, 998);
        assert_eq!(
            testnet.proxy.to_string(),
            "0x1E5439257A314d11e6b0779297Ca92887Ed56087"
        );
        assert_eq!(testnet.deploy_block, 67_938_405);
        let mainnet = bridge(Network::Mainnet);
        assert_eq!(mainnet.chain_id, 999);
        assert_eq!(
            mainnet.proxy.to_string(),
            "0x2940FcBb3C32901Df405da0E96fd97D1E2a53f34"
        );
        assert_eq!(mainnet.deploy_block, 49_937_745);
    }

    #[test]
    fn instrument_is_the_registrar_and_the_ticker() {
        let instrument = instrument(Network::Testnet);
        assert_eq!(instrument.admin, registrar(Network::Testnet));
        assert_eq!(instrument.id, "CHYPE");
    }

    #[test]
    fn client_config_binds_the_chype_instrument_and_registry_of_the_network() {
        let keycloak = KeycloakConfig {
            client_id: "client".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            url: "https://keycloak.example/token".to_string(),
        };
        let config = client_config(
            Network::Mainnet,
            "https://ledger.example".to_string(),
            "alice::1220".to_string(),
            keycloak,
            TokenStandardVersion::V2,
        );
        assert_eq!(config.ledger_host, "https://ledger.example");
        assert_eq!(config.registry_url, Network::Mainnet.registry_url());
        assert_eq!(config.instrument, instrument(Network::Mainnet));
        assert_eq!(config.party, "alice::1220");
        assert_eq!(config.keycloak.client_id, "client");
        assert_eq!(config.version, TokenStandardVersion::V2);
    }

    #[test]
    fn info_maps_each_registrar_back_to_its_network() {
        for network in Network::ALL {
            assert_eq!(INFO.network_of(registrar(network)), Some(network));
        }
        assert_eq!(INFO.ticker, "CHYPE");
        assert_eq!(INFO.minter_claim, ("hasCHYPERole", "Minter"));
        assert_eq!(INFO.dar_dirs, &["dars/dependencies", "dars/chype"]);
    }

    #[test]
    fn a_destination_must_be_a_checksummed_or_lowercase_evm_address() {
        assert_eq!(Chype::validate_destination(CHECKSUMMED), Ok(()));
        assert_eq!(
            Chype::validate_destination(&CHECKSUMMED.to_lowercase()),
            Ok(())
        );
        assert_eq!(
            Chype::validate_destination("0x90f79bf6EB2c4f870365E785982E1f101E93b906").unwrap_err(),
            "address 0x90f79bf6EB2c4f870365E785982E1f101E93b906 does not match its EIP-55 checksum"
        );
        assert_eq!(
            Chype::validate_destination("bcrt1qexample00000").unwrap_err(),
            "address \"bcrt1qexample00000\" is not 0x and 40 hex digits"
        );
    }

    #[test]
    fn the_zero_address_is_not_a_destination() {
        assert_eq!(
            Chype::validate_destination("0x0000000000000000000000000000000000000000").unwrap_err(),
            "destination address is the zero address"
        );
    }

    /// The attestors' own test vectors for the system addresses they refuse.
    #[test]
    fn every_hyperevm_system_address_is_refused() {
        let cases = [
            (
                "0x2222222222222222222222222222222222222222",
                "the HyperEVM HYPE system address",
            ),
            (
                "0x3333333333333333333333333333333333333333",
                "the HyperEVM CoreWriter system contract",
            ),
            (
                "0x0000000000000000000000000000000000000001",
                "a reserved HyperEVM address below 0x10000",
            ),
            (
                "0x0000000000000000000000000000000000000800",
                "a reserved HyperEVM address below 0x10000",
            ),
            (
                "0x000000000000000000000000000000000000ffff",
                "a reserved HyperEVM address below 0x10000",
            ),
            (
                "0x2000000000000000000000000000000000000000",
                "a HyperEVM token system address",
            ),
            (
                "0x20000000000000000000000000000000000000c8",
                "a HyperEVM token system address",
            ),
            (
                "0x200000000000000000000000000000000000ffff",
                "a HyperEVM token system address",
            ),
        ];
        for (address, reason) in cases {
            assert_eq!(
                Chype::validate_destination(address).unwrap_err(),
                format!("destination address {address} is {reason}")
            );
        }
    }

    #[test]
    fn an_ordinary_hyperevm_account_is_a_destination() {
        for address in [
            CHECKSUMMED,
            "0x0000000000000000000000000000000000010000",
            "0x2100000000000000000000000000000000000000",
            "0x2000000000000000000000000000000100000000",
            "0x2222222222222222222222222222222222222223",
        ] {
            assert_eq!(Chype::validate_destination(address), Ok(()), "{address}");
        }
    }

    /// BETH pays to the HyperEVM system addresses, because they are ordinary
    /// accounts on Ethereum.
    #[test]
    fn beth_does_not_refuse_the_hyperevm_system_addresses() {
        use crate::tokens::beth::Beth;
        assert_eq!(
            Beth::validate_destination("0x2222222222222222222222222222222222222222"),
            Ok(())
        );
    }

    fn deposit_args(id: Value) -> Value {
        json!({
            "id": id, "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "instrument": {"admin": registrar(Network::Devnet), "id": "CHYPE"},
            "lastProcessedBlock": "66360900",
            "limits": {"minAmount": "0.25", "maxAmount": "2500"},
        })
    }

    #[test]
    fn a_chype_deposit_account_parses_its_hyperevm_block() {
        let account = DepositAccount::<Chype>::from_active_contract(&active_contract_of(
            DA,
            "00da",
            deposit_args(json!("00orig")),
        ))
        .unwrap();
        assert_eq!(account.last_processed_block, 66_360_900);
        assert_eq!(account.account_id(), "00orig");
        assert_eq!(account.registrar, registrar(Network::Devnet));
    }

    #[test]
    fn a_chype_deposit_account_without_its_block_field_fails_by_name() {
        let mut args = deposit_args(json!(null));
        args.as_object_mut().unwrap().remove("lastProcessedBlock");
        let error =
            DepositAccount::<Chype>::from_active_contract(&active_contract_of(DA, "00da", args))
                .unwrap_err();
        assert_eq!(error, "Missing or invalid 'lastProcessedBlock' field");
    }

    #[test]
    fn a_chype_withdraw_account_parses_its_destination() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "id": null, "destinationAddress": CHECKSUMMED, "pendingBalance": "0", "limits": null,
        });
        let account =
            redeem::WithdrawAccount::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert_eq!(account.destination_address, CHECKSUMMED);
        assert_eq!(account.pending_balance, DamlDecimal::ZERO);
    }

    #[test]
    fn a_chype_withdraw_request_parses_with_or_without_its_source_account() {
        let with_source = json!({
            "owner": "alice", "registrar": registrar(Network::Devnet), "amount": "0.25",
            "destinationAddress": CHECKSUMMED, "sourceAccountId": "00wa",
        });
        let request = WithdrawRequest::<Chype>::from_active_contract(&active_contract_of(
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
            "destinationAddress": CHECKSUMMED, "sourceAccountId": null,
        });
        let request = WithdrawRequest::<Chype>::from_active_contract(&active_contract_of(
            WR,
            "00wr",
            without_source,
        ))
        .unwrap();
        assert_eq!(request.source_account_id, None);
    }

    #[test]
    fn minter_credential_cids_keep_the_chype_claim_of_the_network_registrar() {
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
            credential("00chype", "hasCHYPERole"),
            credential("00beth", "hasBETHRole"),
        ];
        assert_eq!(
            minter_credential_cids(Network::Testnet, &credentials),
            vec!["00chype".to_string()]
        );
        assert!(minter_credential_cids(Network::Mainnet, &credentials).is_empty());
    }

    #[test]
    fn minter_credential_offers_keep_the_chype_claim_of_the_network_registrar() {
        use token::credentials::{Claim, CredentialOffer};
        let offer = |cid: &str, property: &str| CredentialOffer {
            contract_id: cid.to_string(),
            template_id: String::new(),
            created_event_blob: String::new(),
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
        let offers = [
            offer("00chype", "hasCHYPERole"),
            offer("00beth", "hasBETHRole"),
        ];
        let kept: Vec<&str> = minter_credential_offers(Network::Testnet, &offers)
            .into_iter()
            .map(|offer| offer.contract_id.as_str())
            .collect();
        assert_eq!(kept, vec!["00chype"]);
        assert!(minter_credential_offers(Network::Mainnet, &offers).is_empty());
    }

    /// The destination check runs before any request. The test has no
    /// network; an HTTP call would fail with a different message.
    #[tokio::test]
    async fn create_withdraw_account_refuses_a_system_address_before_any_request() {
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
            destination_address: "0x2222222222222222222222222222222222222222".to_string(),
            credential_cids: vec!["00cred".to_string()],
        };
        let error = match redeem::create_withdraw_account(params).await {
            Ok(_) => panic!("a system address must be refused"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "destination address 0x2222222222222222222222222222222222222222 is the HyperEVM HYPE system address"
        );
    }

    /// The registrar check runs before any request. The test has no network;
    /// an HTTP call would fail with a different message.
    #[tokio::test]
    async fn submit_withdraw_refuses_an_account_from_another_network() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": registrar(Network::Devnet),
            "destinationAddress": CHECKSUMMED, "pendingBalance": "0", "limits": null,
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

    /// Produced by the attestors' own deposit-call encoder for the id `smoke`.
    const SMOKE_CALL: &str = "0xdeec7c8f00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000005736d6f6b65000000000000000000000000000000000000000000000000000000";

    fn wei(text: &str) -> mint::U256 {
        text.parse().unwrap()
    }

    fn devnet_deposit_account(contract_id: &str, id: Value, limits: Value) -> mint::DepositAccount {
        let mut args = deposit_args(id);
        args["limits"] = limits;
        DepositAccount::<Chype>::from_active_contract(&active_contract_of(DA, contract_id, args))
            .unwrap()
    }

    #[test]
    fn deposit_call_targets_the_hyperevm_bridge_of_the_network() {
        let account = devnet_deposit_account("smoke", json!(null), json!(null));
        let call =
            mint::deposit_call(Network::Devnet, &account, wei("250000000000000000")).unwrap();
        assert_eq!(call.data.to_string(), SMOKE_CALL);
        assert_eq!(call.deposit_id, "smoke");
        assert_eq!(call.chain_id, 998);
        assert_eq!(
            call.to.to_string(),
            "0xfBC38458ffb91D6Fd731151D46345E89ffc42358"
        );
        assert_eq!(call.value, wei("250000000000000000"));
    }

    #[test]
    fn deposit_call_encodes_the_stable_id_not_the_contract_id() {
        let account = devnet_deposit_account("00current", json!("00orig"), json!(null));
        let call = mint::deposit_call(Network::Devnet, &account, wei("100000000")).unwrap();
        assert_eq!(call.deposit_id, "00orig");
    }

    #[test]
    fn deposit_call_refuses_an_account_from_another_network() {
        let account = devnet_deposit_account("00da", json!(null), json!(null));
        assert_eq!(
            mint::deposit_call(Network::Mainnet, &account, wei("100000000")).unwrap_err(),
            format!(
                "network mismatch: account registrar {}, expected {}",
                registrar(Network::Devnet),
                registrar(Network::Mainnet)
            )
        );
    }

    #[test]
    fn deposit_call_refuses_an_amount_that_is_not_a_whole_unit() {
        let account = devnet_deposit_account("00da", json!(null), json!(null));
        assert_eq!(
            mint::deposit_call(Network::Devnet, &account, wei("100000001")).unwrap_err(),
            "amount 100000001 wei is not a whole multiple of 100000000 wei"
        );
    }

    #[test]
    fn deposit_call_checks_the_account_limits() {
        let limits = json!({"minAmount": "0.25", "maxAmount": "2500"});
        let account = devnet_deposit_account("00da", json!(null), limits);
        assert_eq!(
            mint::deposit_call(Network::Devnet, &account, wei("100000000000000000")).unwrap_err(),
            "Deposit amount 0.1 is below minimum 0.25"
        );
        assert!(mint::deposit_call(Network::Devnet, &account, wei("250000000000000000")).is_ok());
    }

    #[test]
    fn deposit_target_names_the_chain_the_proxy_and_the_id_bytes() {
        let account = devnet_deposit_account("00da", json!("00orig"), json!(null));
        let target = mint::deposit_target(Network::Testnet, &account);
        assert_eq!(
            target.unwrap_err(),
            format!(
                "network mismatch: account registrar {}, expected {}",
                registrar(Network::Devnet),
                registrar(Network::Testnet)
            )
        );
        let target = mint::deposit_target(Network::Devnet, &account).unwrap();
        assert_eq!(target.chain_id, 998);
        assert_eq!(target.to, bridge(Network::Devnet).proxy);
        assert_eq!(target.deposit_id, "00orig");
        assert_eq!(target.deposit_id_bytes.as_ref(), b"00orig");
        assert_eq!(target.deposit_unit_wei, wei("100000000"));
    }
}
