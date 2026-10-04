//! The CBTC localnet suite: one test that runs the phases in order over one
//! fixture, because each phase uses the state the previous one left.

use crate::{
    localnet::fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION},
    tokens::cbtc::Cbtc,
};

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
}
