//! Read-only checks that the deployed BitSafe API still returns the shapes
//! this crate deserializes. They need no credentials and no ledger.

use std::time::Duration;

use crate::{
    DamlDecimal, Network,
    flows::canton_bridge_v1::{CantonBridgeV1, deposit::get_account_contract_rules},
    kits::{
        bitsafe_api::{self, TokenStandardContracts},
        canton::template_suffix,
    },
    tokens::{beth::Beth, cbtc::Cbtc},
};

async fn check_asset<A: CantonBridgeV1>(network: Network) {
    let api = network.bitsafe_api_url();

    let rules = get_account_contract_rules::<A>(api)
        .await
        .expect("account-contract-rules");
    for (rule, template) in [
        (&rules.da_rules, A::DEPOSIT_ACCOUNT_RULES),
        (&rules.wa_rules, A::WITHDRAW_ACCOUNT_RULES),
    ] {
        assert!(!rule.contract_id.is_empty(), "{network}: empty contract id");
        assert!(!rule.created_event_blob.is_empty(), "{network}: empty blob");
        let suffix = template_suffix(template);
        assert!(
            rule.template_id.ends_with(suffix),
            "{network}: {} does not end with {suffix}",
            rule.template_id
        );
    }

    let contracts: TokenStandardContracts<A> =
        bitsafe_api::get_json(api, &format!("{}/v1/token-standard-contracts", A::API_PATH))
            .await
            .expect("token-standard-contracts");
    assert!(!contracts.burn_mint_factory.contract_id.is_empty());
    assert!(!contracts.instrument_configuration.contract_id.is_empty());

    // The endpoint answers 503 until its cache is warm; retry for a minute.
    let path = format!("{}/v1/total-supply", A::API_PATH);
    let mut last_error = String::new();
    for _ in 0..12 {
        match bitsafe_api::get_json::<serde_json::Value>(api, &path).await {
            Ok(body) => {
                DamlDecimal::parse(&body.to_string()).unwrap_or_else(|e| {
                    panic!("{network}: total-supply {body} is not a decimal: {e}")
                });
                return;
            }
            Err(error) if error.contains("503") => {
                last_error = error;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(error) => panic!("{network}: total-supply: {error}"),
        }
    }
    panic!("{network}: total-supply stayed unavailable: {last_error}");
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn cbtc_devnet() {
    check_asset::<Cbtc>(Network::Devnet).await;
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn cbtc_testnet() {
    check_asset::<Cbtc>(Network::Testnet).await;
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn cbtc_mainnet() {
    check_asset::<Cbtc>(Network::Mainnet).await;
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn beth_devnet() {
    check_beth(Network::Devnet).await;
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn beth_testnet() {
    check_beth(Network::Testnet).await;
}

#[tokio::test]
#[ignore = "calls the deployed BitSafe API; run with --ignored"]
async fn beth_mainnet() {
    check_beth(Network::Mainnet).await;
}

/// Runs the BETH checks unless CI lists the network in
/// `BETH_LIVE_API_SKIP`, a comma-separated list of the networks whose
/// BitSafe API does not serve the BETH routes yet.
async fn check_beth(network: Network) {
    let skipped = std::env::var("BETH_LIVE_API_SKIP").unwrap_or_default();
    if skipped
        .split(',')
        .any(|name| name.trim() == network.to_string())
    {
        eprintln!("skipped: BETH_LIVE_API_SKIP lists {network}");
        return;
    }
    check_asset::<Beth>(network).await;
}
