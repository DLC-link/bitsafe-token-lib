//! Canton plumbing that every bridge flow repeats: list contracts by template,
//! exercise a choice with disclosed contracts, find a created event, and read
//! fields from a create argument. Nothing here knows about deposits or
//! withdrawals.

use common::{instrument::InstrumentId, submission, transfer::DisclosedContract};
use ledger::{
    active_contracts,
    common::{IdentifierFilter, TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter},
    ledger_end,
    models::{CreatedEvent, Event, JsActiveContract, JsSubmitAndWaitForTransactionResponse},
    submit,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use token::holding::Holding;

use crate::DamlDecimal;

/// The utility registry's holding template. Every asset's holdings use it.
pub(crate) const HOLDING_TEMPLATE_ID: &str =
    "#utility-registry-holding-v0:Utility.Registry.Holding.V0.Holding:Holding";

/// The transaction limits on a deposit or withdraw account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limits {
    #[serde(rename = "minAmount")]
    pub min_amount: Option<DamlDecimal>,
    #[serde(rename = "maxAmount")]
    pub max_amount: Option<DamlDecimal>,
}

/// Checks an amount against an account's limits. No limits means any amount.
pub(crate) fn check_limits(
    operation: &str,
    amount: DamlDecimal,
    limits: &Option<Limits>,
) -> Result<(), String> {
    if let Some(lim) = limits {
        if let Some(min) = &lim.min_amount
            && amount < *min
        {
            return Err(format!(
                "{} amount {} is below minimum {}",
                operation, amount, min
            ));
        }
        if let Some(max) = &lim.max_amount
            && amount > *max
        {
            return Err(format!(
                "{} amount {} exceeds maximum {}",
                operation, amount, max
            ));
        }
    }
    Ok(())
}

/// The party's active contracts of one template, at the current ledger end.
pub(crate) async fn list_by_template(
    ledger_host: &str,
    party: &str,
    access_token: &str,
    template_id: &str,
) -> Result<Vec<JsActiveContract>, String> {
    let end = ledger_end::get(ledger_end::Params {
        access_token: access_token.to_string(),
        ledger_host: ledger_host.to_string(),
    })
    .await?;
    let filter = IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
        template_filter: TemplateFilter {
            value: TemplateFilterValue {
                template_id: Some(template_id.to_string()),
                include_created_event_blob: true,
            },
        },
    });
    active_contracts::get_by_party(active_contracts::Params {
        ledger_host: ledger_host.to_string(),
        party: party.to_string(),
        filter,
        access_token: access_token.to_string(),
        ledger_end: end.offset,
        unknown_contract_entry_handler: None,
    })
    .await
}

/// One choice to exercise, with the contracts the ledger must see disclosed.
pub(crate) struct Exercise<'a> {
    pub template_id: &'a str,
    pub contract_id: &'a str,
    pub choice: &'a str,
    pub argument: Value,
    pub disclosed: Vec<DisclosedContract>,
}

/// The user id the localnet suite sends with each submission. The localnet
/// sandbox runs without login, so it reads the user id from the request body.
/// Production reads the user from the access token instead.
#[cfg(test)]
pub(crate) static TEST_USER_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();

#[cfg(test)]
fn submission_user_id() -> Option<String> {
    TEST_USER_ID.get().cloned()
}

#[cfg(not(test))]
fn submission_user_id() -> Option<String> {
    None
}

/// Exercises one choice as `party` and waits for the transaction.
pub(crate) async fn exercise(
    ledger_host: &str,
    party: &str,
    access_token: &str,
    command: Exercise<'_>,
) -> Result<JsSubmitAndWaitForTransactionResponse, String> {
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: command.template_id.to_string(),
            contract_id: command.contract_id.to_string(),
            choice: command.choice.to_string(),
            choice_argument: submission::ChoiceArgumentsVariations::Generic(command.argument),
        },
    };
    let request = submission::Submission {
        act_as: vec![party.to_string()],
        read_as: None,
        command_id: format!("cmd-{}", uuid::Uuid::new_v4()),
        disclosed_contracts: command.disclosed,
        commands: vec![submission::Command::ExerciseCommand(exercise_command)],
        user_id: submission_user_id(),
        ..Default::default()
    };
    let raw = submit::wait_for_transaction(submit::Params {
        ledger_host: ledger_host.to_string(),
        access_token: access_token.to_string(),
        request,
    })
    .await?;
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse submit response: {}", e))
}

/// A contract to disclose, on any synchronizer.
pub(crate) fn disclosed(
    contract_id: &str,
    template_id: &str,
    created_event_blob: &str,
) -> DisclosedContract {
    DisclosedContract {
        contract_id: contract_id.to_string(),
        created_event_blob: created_event_blob.to_string(),
        template_id: Some(template_id.to_string()),
        synchronizer_id: String::new(),
    }
}

/// The module-and-entity part of a template id, from the first colon on.
/// `#cbtc:CBTC.DepositAccount:CBTCDepositAccount` gives
/// `:CBTC.DepositAccount:CBTCDepositAccount`, which also ends the package-id
/// form the ledger returns.
pub(crate) fn template_suffix(template_id: &str) -> &str {
    template_id
        .find(':')
        .map_or(template_id, |at| &template_id[at..])
}

/// The created event inside an event. The OpenAPI generator names the
/// `oneOf` variants by position; `EventOneOf1` is the created event.
fn as_created_event(event: &Event) -> Option<&CreatedEvent> {
    match event {
        Event::EventOneOf1(wrapper) => Some(&wrapper.created_event),
        _ => None,
    }
}

/// The first created event whose template id ends with `suffix`.
pub(crate) fn created_by_suffix<'a>(
    response: &'a JsSubmitAndWaitForTransactionResponse,
    suffix: &str,
) -> Option<&'a CreatedEvent> {
    response
        .transaction
        .events
        .iter()
        .filter_map(as_created_event)
        .find(|created| created.template_id.ends_with(suffix))
}

/// A created event in the shape the model parsers read.
pub(crate) fn as_active_contract(created: &CreatedEvent) -> JsActiveContract {
    JsActiveContract {
        created_event: Box::new(created.clone()),
        reassignment_counter: 0,
        synchronizer_id: String::new(),
    }
}

/// The create argument of a contract, as a JSON object.
pub(crate) fn create_args(contract: &JsActiveContract) -> Result<&Map<String, Value>, String> {
    contract
        .created_event
        .create_argument
        .as_ref()
        .and_then(|v| v.as_object())
        .ok_or_else(|| "createArgument is not an object".to_string())
}

/// A text field that must be present.
pub(crate) fn required_str(args: &Map<String, Value>, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("Missing '{}' field", field))
}

/// A text field that Daml declares `Optional`.
pub(crate) fn optional_str(args: &Map<String, Value>, field: &str) -> Option<String> {
    args.get(field).and_then(|v| v.as_str()).map(str::to_string)
}

/// The `limits` field, absent or `null` meaning no limits.
pub(crate) fn optional_limits(args: &Map<String, Value>) -> Result<Option<Limits>, String> {
    match args.get("limits") {
        None => Ok(None),
        Some(v) if v.is_null() => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| format!("Invalid 'limits' field: {}", e)),
    }
}

/// The parameters for listing a party's holdings of one instrument.
pub struct ListHoldingsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    /// The instrument, admin included. Two registrars can both issue `CBTC`.
    pub instrument_id: InstrumentId,
}

/// Lists a party's unlocked holdings of one instrument.
///
/// The filter compares the whole instrument, admin included. A holding with
/// the right ticker under a foreign registrar is a different instrument.
///
/// # Errors
///
/// Fails on a ledger error, and on any holding that does not parse: a loud
/// error beats a silently short list.
pub async fn list_holdings(params: ListHoldingsParams) -> Result<Vec<Holding>, String> {
    let contracts = list_by_template(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        HOLDING_TEMPLATE_ID,
    )
    .await?;
    select_holdings(&contracts, &params.instrument_id)
}

/// The unlocked holdings of `instrument` among the party's holding contracts.
/// Split out of [`list_holdings`] so a unit test reaches it without a ledger.
fn select_holdings(
    contracts: &[JsActiveContract],
    instrument: &InstrumentId,
) -> Result<Vec<Holding>, String> {
    let holdings = contracts
        .iter()
        .filter(|contract| !Holding::is_locked_in_contract(contract))
        .map(Holding::from_active_contract)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(holdings
        .into_iter()
        .filter(|holding| holding.instrument_id == *instrument)
        .collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_fixtures::{
        active_contract, created_event_value, exercised_event_value, transaction_response,
    };

    fn d(s: &str) -> DamlDecimal {
        DamlDecimal::parse(s).unwrap()
    }

    fn limits(min: Option<&str>, max: Option<&str>) -> Option<Limits> {
        Some(Limits {
            min_amount: min.map(d),
            max_amount: max.map(d),
        })
    }

    #[test]
    fn check_limits_passes_without_limits_and_inside_the_bounds() {
        assert!(check_limits("Withdraw", d("1"), &None).is_ok());
        assert!(check_limits("Withdraw", d("1"), &limits(Some("0.001"), Some("10"))).is_ok());
        assert!(check_limits("Withdraw", d("1"), &limits(Some("1"), Some("10"))).is_ok());
        assert!(check_limits("Withdraw", d("10"), &limits(Some("1"), Some("10"))).is_ok());
    }

    #[test]
    fn check_limits_names_the_bound_that_fails() {
        let below = check_limits("Withdraw", d("0.001"), &limits(Some("0.01"), None));
        assert_eq!(
            below.unwrap_err(),
            "Withdraw amount 0.001 is below minimum 0.01"
        );
        let above = check_limits("Deposit", d("10"), &limits(None, Some("5")));
        assert_eq!(above.unwrap_err(), "Deposit amount 10 exceeds maximum 5");
    }

    #[test]
    fn limits_deserialize_from_the_ledger_shape() {
        let parsed: Limits =
            serde_json::from_value(json!({"minAmount": "0.001", "maxAmount": null})).unwrap();
        assert_eq!(
            parsed,
            Limits {
                min_amount: Some(d("0.001")),
                max_amount: None
            }
        );
        let bad: Result<Limits, _> =
            serde_json::from_value(json!({"minAmount": "not_a_number", "maxAmount": null}));
        assert!(bad.is_err());
    }

    #[test]
    fn template_suffix_strips_the_package_part() {
        assert_eq!(
            template_suffix("#cbtc:CBTC.DepositAccount:CBTCDepositAccount"),
            ":CBTC.DepositAccount:CBTCDepositAccount"
        );
        assert_eq!(template_suffix("no-colon"), "no-colon");
    }

    #[test]
    fn created_by_suffix_finds_the_created_event_and_skips_the_rest() {
        let response = transaction_response(
            "tx-1",
            json!([
                exercised_event_value(
                    "pkg:CBTC.DepositAccount:CBTCDepositAccountRules",
                    "CBTCDepositAccountRules_CreateDepositAccount",
                    json!(null),
                ),
                created_event_value("pkg:Some.Other:Template", "00other", json!(null)),
                created_event_value(
                    "f240dd5d:CBTC.DepositAccount:CBTCDepositAccount",
                    "000b5aff",
                    json!(null),
                ),
            ]),
        );
        let found = created_by_suffix(&response, ":CBTC.DepositAccount:CBTCDepositAccount");
        assert_eq!(found.unwrap().contract_id, "000b5aff");
        assert!(
            created_by_suffix(&response, ":CBTC.WithdrawAccount:CBTCWithdrawAccount").is_none()
        );
        assert!(created_by_suffix(&transaction_response("tx-2", json!(null)), ":X").is_none());
    }

    #[test]
    fn field_readers_report_the_missing_field_by_name() {
        let contract = active_contract("00a", json!({"owner": "alice", "limits": null}));
        let args = create_args(&contract).unwrap();
        assert_eq!(required_str(args, "owner").unwrap(), "alice");
        assert_eq!(
            required_str(args, "operator").unwrap_err(),
            "Missing 'operator' field"
        );
        assert_eq!(optional_str(args, "id"), None);
        assert_eq!(optional_limits(args).unwrap(), None);
        let not_object = active_contract("00b", json!("text"));
        assert_eq!(
            create_args(&not_object).unwrap_err(),
            "createArgument is not an object"
        );
    }

    fn holding_payload(admin: &str, ticker: &str, lock: Value) -> Value {
        json!({
            "operator": "operator::1220aa",
            "provider": "provider::1220bb",
            "registrar": admin,
            "owner": "alice::1220cc",
            "instrument": {
                "source": admin,
                "id": ticker,
                "scheme": "RegistrarInternalScheme",
            },
            "label": "",
            "amount": "1.0",
            "lock": lock,
        })
    }

    fn cbtc() -> InstrumentId {
        InstrumentId {
            admin: "cbtc-network::1220ab".to_string(),
            id: "CBTC".to_string(),
        }
    }

    #[test]
    fn select_holdings_keeps_only_unlocked_holdings_of_the_instrument() {
        let contracts = vec![
            active_contract(
                "00keep",
                holding_payload("cbtc-network::1220ab", "CBTC", json!(null)),
            ),
            active_contract(
                "00foreign",
                holding_payload("attacker::1220ff", "CBTC", json!(null)),
            ),
            active_contract(
                "00ticker",
                holding_payload("cbtc-network::1220ab", "BETH", json!(null)),
            ),
            active_contract(
                "00locked",
                holding_payload(
                    "cbtc-network::1220ab",
                    "CBTC",
                    json!({"holders": [], "context": null}),
                ),
            ),
        ];
        let kept: Vec<String> = select_holdings(&contracts, &cbtc())
            .unwrap()
            .into_iter()
            .map(|h| h.contract_id)
            .collect();
        assert_eq!(kept, vec!["00keep".to_string()]);
    }

    #[test]
    fn select_holdings_fails_on_one_unparseable_holding() {
        let contracts = vec![
            active_contract(
                "00good",
                holding_payload("cbtc-network::1220ab", "CBTC", json!(null)),
            ),
            active_contract("00bad", json!({"owner": "alice::1220cc"})),
        ];
        assert!(select_holdings(&contracts, &cbtc()).is_err());
    }
}
