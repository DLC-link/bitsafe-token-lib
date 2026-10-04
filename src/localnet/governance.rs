//! Runs a governance action through the one-member body: create, confirm,
//! execute. Each step is its own submission, because the next step needs the
//! contract id the previous step created.

use canton_api_client::models::JsSubmitAndWaitForTransactionResponse;
use serde_json::{Value, json};

use crate::localnet::{
    fixture::{Fixture, GOVERNANCE_RULES},
    ledger::created_cid,
};

const GOVERNANCE_CONFIRMATION: &str =
    "#governance-core-v1:Governance.Confirmation:GovernanceConfirmation";

/// The registrar proposes the action `action_template` with `arguments`,
/// confirms it as the only member, and executes it. Returns the execute
/// transaction.
///
/// The registrar plays every role: governance party, proposer, confirmer
/// and executor.
pub(crate) async fn run_action(
    fixture: &Fixture,
    action_template: &str,
    arguments: Value,
) -> Result<JsSubmitAndWaitForTransactionResponse, String> {
    let registrar = fixture.registrar.as_str();
    let action = fixture
        .ledger
        .create(&[registrar], action_template, arguments)
        .await?;
    let response = fixture
        .ledger
        .exercise(
            &[registrar],
            GOVERNANCE_RULES,
            &fixture.governance_rules,
            "GovernanceRules_ConfirmAction",
            json!({"confirmer": registrar, "actionProposalCid": action}),
            &[],
        )
        .await?;
    let confirmation = created_cid(&response, GOVERNANCE_CONFIRMATION)?;
    // Execute archives the action and the confirmation. It leaves the rules
    // contract active, so the fixture's rules cid stays valid.
    fixture
        .ledger
        .exercise(
            &[registrar],
            GOVERNANCE_RULES,
            &fixture.governance_rules,
            "GovernanceRules_ExecuteConfirmedAction",
            json!({
                "executor": registrar,
                "actionProposalCid": action,
                "confirmations": [confirmation],
            }),
            &[],
        )
        .await
}
