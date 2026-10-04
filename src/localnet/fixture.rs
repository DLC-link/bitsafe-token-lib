//! The test registrar and test user of one suite run, and the contracts the
//! registrar owns.

use serde_json::json;

use crate::localnet::ledger::Ledger;

pub(crate) const GOVERNANCE_RULES: &str = "#governance-core-v1:Governance.Rules:GovernanceRules";

pub(crate) struct Fixture {
    pub(crate) ledger: Ledger,
    #[expect(
        dead_code,
        reason = "the BETH localnet suite reads it; the CBTC phases do not"
    )]
    pub(crate) operator: String,
    #[expect(
        dead_code,
        reason = "the BETH localnet suite reads it; the CBTC phases do not"
    )]
    pub(crate) dso: String,
    /// The fresh plain party that plays the CBTC registrar.
    pub(crate) registrar: String,
    /// The fresh party that owns the accounts and holdings.
    #[expect(dead_code, reason = "the CBTC phases after the fixture read it")]
    pub(crate) user: String,
    /// The one-member governance body of `registrar`.
    pub(crate) governance_rules: String,
}

impl Fixture {
    /// Finds the sandbox's operator and dso, allocates a fresh registrar and
    /// user, and gives the registrar a governance body with itself as the
    /// only member.
    pub(crate) async fn new() -> Result<Fixture, String> {
        let ledger = Ledger::from_env();
        let operator = ledger.party("operator").await?;
        let dso = ledger.party("dso").await?;
        let registrar = ledger.allocate_party("testreg").await?;
        let user = ledger.allocate_party("testuser").await?;
        // Daml's `Set Party` is a map from party to unit, `Int` is a string,
        // and `RelTime` is a count of microseconds.
        let governance_rules = ledger
            .create(
                &[&registrar],
                GOVERNANCE_RULES,
                json!({
                    "governanceParty": registrar,
                    "members": {"map": [[registrar, {}]]},
                    "threshold": "1",
                    "actionConfirmationTimeout": {"microseconds": "1800000000"},
                    "additionalProposers": null,
                }),
            )
            .await?;
        Ok(Fixture {
            ledger,
            operator,
            dso,
            registrar,
            user,
            governance_rules,
        })
    }
}
