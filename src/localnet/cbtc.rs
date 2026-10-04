//! The CBTC localnet suite: one test that runs the phases in order over one
//! fixture, because each phase uses the state the previous one left.

use crate::localnet::fixture::{Fixture, GOVERNANCE_RULES};

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
}
