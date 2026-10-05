//! The localnet integration suite. It runs the CBTC and then the BETH flows
//! against canton's published sandbox image, with one fresh plain test
//! registrar per run. Start the sandbox with `docker compose -f
//! localnet/docker-compose.yml up -d --wait`, then run `cargo test --lib
//! localnet -- --ignored --nocapture --test-threads=1`.

mod beth;
mod cbtc;
mod fixture;
mod governance;
mod ledger;
mod phases;

use fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION};

/// One test runs every phase in a fixed order over one fixture, because each
/// phase uses the state the previous one left.
#[tokio::test]
#[ignore = "needs the localnet sandbox; see src/localnet/mod.rs"]
async fn localnet_suite() {
    let fixture = Fixture::new().await.expect("fixture");
    let rules = fixture
        .ledger
        .active(&fixture.registrar, GOVERNANCE_RULES)
        .await
        .expect("governance rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].created_event.contract_id, fixture.governance_rules);
    assert!(!fixture.allocation_factory.created_event_blob.is_empty());

    cbtc::run(&fixture).await;
    beth::run(&fixture).await;

    // One registrar service configured one instrument per asset.
    let instrument_configs = fixture
        .ledger
        .active(&fixture.registrar, INSTRUMENT_CONFIGURATION)
        .await
        .expect("instrument configurations");
    assert_eq!(instrument_configs.len(), 2);
}
