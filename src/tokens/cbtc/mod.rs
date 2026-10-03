//! CBTC: Bitcoin bridged to Canton. This module is the whole CBTC API.

use crate::{
    AssetInfo, InstrumentId, KeycloakConfig, Network, TokenClientConfig, TokenStandardVersion,
    flows::canton_bridge_v1::CantonBridgeV1,
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

impl crate::flows::canton_bridge_v1::models::WithdrawRequest<Cbtc> {
    /// The Bitcoin transaction id of the payout, which the registrar sets
    /// when it creates the request.
    ///
    /// # Errors
    ///
    /// Fails when the request carries no `btcTxId`, as `cbtc-lib` did.
    #[allow(dead_code)]
    pub fn btc_tx_id(&self) -> Result<&str, String> {
        self.create_argument
            .get("btcTxId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'btcTxId' field".to_string())
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
}
