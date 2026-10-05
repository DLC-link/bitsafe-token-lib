//! The localnet integration suite. It checks each asset's DAR set, then runs
//! the CBTC and then the BETH flows against canton's published sandbox image,
//! with one fresh plain test registrar per run. Start the sandbox with
//! `docker compose -f localnet/docker-compose.yml up -d --wait`, then run
//! `cargo test --lib localnet -- --ignored --nocapture --test-threads=1`.

mod beth;
mod cbtc;
mod fixture;
mod governance;
mod ledger;
mod phases;

use std::path::Path;

use fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION};

use crate::{check_dars, dar_check::DarCheckStatus, tokens};

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

    // The sandbox holds the newest package of every DAR each asset ships.
    for info in tokens::ALL {
        let result = check_dars(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            info.dar_dirs,
            fixture.ledger.host.clone(),
            fixture.ledger.token.clone(),
        )
        .await
        .expect("check_dars");
        assert_eq!(
            result.status,
            DarCheckStatus::Pass,
            "{} lacks {:?}",
            info.ticker,
            result.missing
        );
    }

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
