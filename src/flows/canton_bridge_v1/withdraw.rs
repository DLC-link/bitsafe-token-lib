//! The withdraw flows of the family: accounts, the burn and the payout
//! records. The burn checks locally what the Daml choice checks, so a bad
//! call fails with a clear message instead of an opaque Canton rejection.

use ledger::models::JsSubmitAndWaitForTransactionResponse;
use serde_json::{Map, Value, json};
use token::holding::Holding;

use crate::{
    DamlDecimal, InstrumentId,
    flows::canton_bridge_v1::{
        CantonBridgeV1, check_registrar, created_contract_id,
        models::{WithdrawAccount, WithdrawRequest},
    },
    kits::{
        bitsafe_api::{self, AccountContractRuleSet, TokenStandardContracts},
        canton::{self, Exercise},
    },
};

/// The parameters for listing a party's withdraw accounts.
pub struct ListWithdrawAccountsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// The parameters for listing a party's withdraw requests.
pub struct ListWithdrawRequestsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// The parameters for creating a withdraw account.
pub struct CreateWithdrawAccountParams<A> {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    /// The rules from `get_account_contract_rules`; this flow discloses `wa_rules`.
    pub account_rules: AccountContractRuleSet<A>,
    /// Where the payout goes. The flow checks it locally and sends it as given.
    pub destination_address: String,
    pub credential_cids: Vec<String>,
}

/// The parameters for a burn.
pub struct SubmitWithdrawParams<'a, A> {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    /// The withdraw account to burn into. Its pending balance must be zero.
    pub account: &'a WithdrawAccount<A>,
    pub amount: DamlDecimal,
    /// The holdings to burn, from `list_holdings`. Their sum must cover `amount`.
    pub holdings: &'a [Holding],
    /// The party's Minter credential cids, from `minter_credential_cids`.
    /// The burn needs at least one.
    pub credential_cids: Vec<String>,
}

/// Lists the party's live withdraw accounts of this asset.
pub(crate) async fn list_withdraw_accounts<A: CantonBridgeV1>(
    params: ListWithdrawAccountsParams,
) -> Result<Vec<WithdrawAccount<A>>, String> {
    canton::list_by_template(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        A::WITHDRAW_ACCOUNT,
    )
    .await?
    .iter()
    .map(WithdrawAccount::from_active_contract)
    .collect()
}

/// Lists the party's withdraw accounts and picks one by contract id.
pub(crate) async fn find_withdraw_account<A: CantonBridgeV1>(
    params: ListWithdrawAccountsParams,
    contract_id: &str,
) -> Result<WithdrawAccount<A>, String> {
    list_withdraw_accounts(params)
        .await?
        .into_iter()
        .find(|account| account.contract_id == contract_id)
        .ok_or_else(|| {
            format!(
                "Withdraw account with contract ID {} not found",
                contract_id
            )
        })
}

/// The choice argument of `CreateWithdrawAccount`, with the asset's
/// destination field name.
pub(crate) fn create_withdraw_account_argument<A: CantonBridgeV1>(
    party: &str,
    destination_address: &str,
    credential_cids: &[String],
) -> Value {
    let mut argument = Map::new();
    argument.insert("owner".to_string(), json!(party));
    argument.insert(
        A::DESTINATION_ADDRESS_FIELD.to_string(),
        json!(destination_address),
    );
    argument.insert("credentialCids".to_string(), json!(credential_cids));
    Value::Object(argument)
}

/// Checks the destination, creates a withdraw account and returns it as the
/// ledger now lists it.
pub(crate) async fn create_withdraw_account<A: CantonBridgeV1>(
    params: CreateWithdrawAccountParams<A>,
) -> Result<WithdrawAccount<A>, String> {
    A::validate_destination(&params.destination_address)?;
    let rules = &params.account_rules.wa_rules;
    let response = canton::exercise(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        Exercise {
            template_id: A::WITHDRAW_ACCOUNT_RULES,
            contract_id: &rules.contract_id,
            choice: A::CREATE_WITHDRAW_ACCOUNT_CHOICE,
            argument: create_withdraw_account_argument::<A>(
                &params.party,
                &params.destination_address,
                &params.credential_cids,
            ),
            disclosed: vec![rules.disclosed()],
        },
    )
    .await?;
    let contract_id = created_contract_id(&response, A::WITHDRAW_ACCOUNT, "WithdrawAccount")?;
    let accounts = list_withdraw_accounts::<A>(ListWithdrawAccountsParams {
        ledger_host: params.ledger_host,
        party: params.party,
        access_token: params.access_token,
    })
    .await?;
    accounts
        .into_iter()
        .find(|account| account.contract_id == contract_id)
        .ok_or_else(|| {
            format!(
                "Created WithdrawAccount {} not found in active contracts",
                contract_id
            )
        })
}

/// Checks a burn locally against the same rules the Daml choice applies.
pub(crate) fn check_withdraw<A: CantonBridgeV1>(
    registrar: &str,
    params: &SubmitWithdrawParams<'_, A>,
) -> Result<(), String> {
    let account = params.account;
    check_registrar(&account.registrar, registrar)?;
    if params.party != account.owner {
        return Err(format!(
            "Party {} is not the owner {} of the withdraw account",
            params.party, account.owner
        ));
    }
    if params.credential_cids.is_empty() {
        return Err("Credential CIDs required".to_string());
    }
    if account.pending_balance != DamlDecimal::ZERO {
        return Err(format!(
            "A withdrawal is in progress: pending balance {}",
            account.pending_balance
        ));
    }
    if params.amount <= DamlDecimal::ZERO {
        return Err("Amount to withdraw must be greater than 0".to_string());
    }
    account.check_amount(params.amount)?;
    if params.holdings.is_empty() {
        return Err("No holdings to withdraw from".to_string());
    }
    let expected = InstrumentId {
        admin: registrar.to_string(),
        id: A::TICKER.to_string(),
    };
    for holding in params.holdings {
        if holding.owner != account.owner {
            return Err(format!(
                "Holding {} is owned by {}, expected {}",
                holding.contract_id, holding.owner, account.owner
            ));
        }
        if holding.instrument_id != expected {
            return Err(format!(
                "Holding {} has instrument {}/{}, expected {}/{}",
                holding.contract_id,
                holding.instrument_id.admin,
                holding.instrument_id.id,
                expected.admin,
                expected.id
            ));
        }
    }
    let total: DamlDecimal = params.holdings.iter().map(|holding| holding.amount).sum();
    if params.amount > total {
        return Err(format!(
            "Amount to withdraw {} is greater than the holdings' total {}",
            params.amount, total
        ));
    }
    Ok(())
}

/// The `Withdraw` exercise for a checked burn.
///
/// The template is the package-name form, so Canton picks the highest vetted
/// package and an account created on an older version accepts the newer
/// argument fields. The amount goes out as a decimal string, because Canton
/// rejects the scientific notation serde can produce for small numbers.
pub(crate) fn withdraw_exercise<'a, A: CantonBridgeV1>(
    params: &'a SubmitWithdrawParams<'_, A>,
    contracts: &TokenStandardContracts<A>,
) -> Exercise<'a> {
    let issuer_credentials: Vec<Value> = contracts
        .issuer_credential
        .iter()
        .map(|credential| json!({"tag": "AV_ContractId", "value": credential.contract_id}))
        .collect();
    let mut context = Map::new();
    context.insert(
        "utility.digitalasset.com/instrument-configuration".to_string(),
        json!({"tag": "AV_ContractId", "value": contracts.instrument_configuration.contract_id}),
    );
    context.insert(
        "utility.digitalasset.com/issuer-credentials".to_string(),
        json!({"tag": "AV_List", "value": issuer_credentials}),
    );
    let holding_cids: Vec<&str> = params
        .holdings
        .iter()
        .map(|holding| holding.contract_id.as_str())
        .collect();
    let argument = json!({
        "tokens": holding_cids,
        "amount": params.amount.to_string(),
        "burnMintFactoryCid": contracts.burn_mint_factory.contract_id,
        "extraArgs": {
            "context": {"values": context},
            "meta": {"values": {"splice.lfdecentralizedtrust.org/reason": A::WITHDRAW_REASON}},
        },
        "credentialCids": params.credential_cids,
    });
    let mut disclosed = vec![
        contracts.burn_mint_factory.disclosed(),
        contracts.instrument_configuration.disclosed(),
    ];
    disclosed.extend(
        contracts
            .issuer_credential
            .iter()
            .map(|credential| credential.disclosed()),
    );
    Exercise {
        template_id: A::WITHDRAW_ACCOUNT,
        contract_id: &params.account.contract_id,
        choice: A::WITHDRAW_CHOICE,
        argument,
        disclosed,
    }
}

/// The withdraw account the `Withdraw` choice recreated with the new
/// pending balance.
pub(crate) fn recreated_withdraw_account<A: CantonBridgeV1>(
    response: &JsSubmitAndWaitForTransactionResponse,
) -> Result<WithdrawAccount<A>, String> {
    let created = canton::created_by_suffix(response, canton::template_suffix(A::WITHDRAW_ACCOUNT))
        .ok_or_else(|| "No updated WithdrawAccount was found in the transaction".to_string())?;
    WithdrawAccount::from_active_contract(&canton::as_active_contract(created))
}

/// Burns the holdings into the withdraw account. The registrar later creates
/// a withdraw request and pays out; this call does not wait for that.
pub(crate) async fn submit_withdraw<A: CantonBridgeV1>(
    registrar: &str,
    api_url: &str,
    params: SubmitWithdrawParams<'_, A>,
) -> Result<WithdrawAccount<A>, String> {
    check_withdraw(registrar, &params)?;
    let contracts: TokenStandardContracts<A> = bitsafe_api::get_json(
        api_url,
        &format!("{}/v1/token-standard-contracts", A::API_PATH),
    )
    .await?;
    let response = canton::exercise(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        withdraw_exercise(&params, &contracts),
    )
    .await?;
    recreated_withdraw_account(&response)
}

/// Lists the registrar-created payout records the party can see.
pub(crate) async fn list_withdraw_requests<A: CantonBridgeV1>(
    params: ListWithdrawRequestsParams,
) -> Result<Vec<WithdrawRequest<A>>, String> {
    canton::list_by_template(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        A::WITHDRAW_REQUEST,
    )
    .await?
    .iter()
    .map(WithdrawRequest::from_active_contract)
    .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        test_fixtures::{active_contract_of, created_event_value_with_blob, transaction_response},
        tokens::cbtc::Cbtc,
    };

    const REGISTRAR: &str = "cbtc-network::1220";

    fn d(s: &str) -> DamlDecimal {
        DamlDecimal::parse(s).unwrap()
    }

    fn account(pending: &str, limits: Value) -> WithdrawAccount<Cbtc> {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": REGISTRAR,
            "destinationBtcAddress": "bcrt1qexample00000", "pendingBalance": pending, "limits": limits,
        });
        WithdrawAccount::from_active_contract(&active_contract_of(
            "f240:CBTC.WithdrawAccount:CBTCWithdrawAccount",
            "00wa",
            args,
        ))
        .unwrap()
    }

    fn holding(cid: &str, admin: &str, ticker: &str, amount: &str) -> Holding {
        Holding {
            contract_id: cid.to_string(),
            amount: d(amount),
            instrument_id: InstrumentId {
                admin: admin.to_string(),
                id: ticker.to_string(),
            },
            owner: "alice".to_string(),
            account_label: String::new(),
        }
    }

    fn params<'a>(
        account: &'a WithdrawAccount<Cbtc>,
        holdings: &'a [Holding],
        amount: &str,
    ) -> SubmitWithdrawParams<'a, Cbtc> {
        SubmitWithdrawParams {
            ledger_host: "https://ledger".to_string(),
            party: "alice".to_string(),
            access_token: "token".to_string(),
            account,
            amount: d(amount),
            holdings,
            credential_cids: vec!["00cred".to_string()],
        }
    }

    #[test]
    fn a_withdraw_inside_every_rule_passes() {
        let account = account("0", json!({"minAmount": "0.001", "maxAmount": "10"}));
        let holdings = [
            holding("00h1", REGISTRAR, "CBTC", "0.4"),
            holding("00h2", REGISTRAR, "CBTC", "0.6"),
        ];
        assert!(check_withdraw(REGISTRAR, &params(&account, &holdings, "1")).is_ok());
    }

    #[test]
    fn a_withdraw_fails_on_the_wrong_network() {
        let account = account("0", json!(null));
        let holdings = [holding("00h", REGISTRAR, "CBTC", "1")];
        assert_eq!(
            check_withdraw("cbtc-network::other", &params(&account, &holdings, "0.5")).unwrap_err(),
            "network mismatch: account registrar cbtc-network::1220, expected cbtc-network::other"
        );
    }

    #[test]
    fn a_withdraw_fails_when_the_caller_is_not_the_account_owner() {
        let account = account("0", json!(null));
        let holdings = [holding("00h", REGISTRAR, "CBTC", "1")];
        let mut p = params(&account, &holdings, "0.5");
        p.party = "bob".to_string();
        assert_eq!(
            check_withdraw(REGISTRAR, &p).unwrap_err(),
            "Party bob is not the owner alice of the withdraw account"
        );
    }

    #[test]
    fn a_withdraw_fails_without_credentials() {
        let account = account("0", json!(null));
        let holdings = [holding("00h", REGISTRAR, "CBTC", "1")];
        let mut p = params(&account, &holdings, "0.5");
        p.credential_cids = Vec::new();
        assert_eq!(
            check_withdraw(REGISTRAR, &p).unwrap_err(),
            "Credential CIDs required"
        );
    }

    #[test]
    fn a_withdraw_fails_on_a_holding_of_another_owner() {
        let account = account("0", json!(null));
        let holdings = [
            holding("00h1", REGISTRAR, "CBTC", "1"),
            Holding {
                owner: "bob".to_string(),
                ..holding("00h2", REGISTRAR, "CBTC", "1")
            },
        ];
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0.5")).unwrap_err(),
            "Holding 00h2 is owned by bob, expected alice"
        );
    }

    #[test]
    fn a_second_withdraw_fails_while_a_payout_is_pending() {
        let account = account("0.2", json!(null));
        let holdings = [holding("00h", REGISTRAR, "CBTC", "1")];
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0.5")).unwrap_err(),
            "A withdrawal is in progress: pending balance 0.2"
        );
    }

    #[test]
    fn a_withdraw_fails_on_a_zero_amount_and_outside_the_limits() {
        let account = account("0", json!({"minAmount": "0.01", "maxAmount": null}));
        let holdings = [holding("00h", REGISTRAR, "CBTC", "1")];
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0")).unwrap_err(),
            "Amount to withdraw must be greater than 0"
        );
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0.001")).unwrap_err(),
            "Withdraw amount 0.001 is below minimum 0.01"
        );
    }

    #[test]
    fn a_withdraw_fails_on_a_holding_of_another_instrument() {
        let account = account("0", json!(null));
        let holdings = [holding("00h", "beth-network::1220", "BETH", "1")];
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0.5")).unwrap_err(),
            "Holding 00h has instrument beth-network::1220/BETH, expected cbtc-network::1220/CBTC"
        );
    }

    #[test]
    fn a_withdraw_fails_when_the_holdings_do_not_cover_the_amount() {
        let account = account("0", json!(null));
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &[], "0.5")).unwrap_err(),
            "No holdings to withdraw from"
        );
        let holdings = [holding("00h", REGISTRAR, "CBTC", "0.3")];
        assert_eq!(
            check_withdraw(REGISTRAR, &params(&account, &holdings, "0.5")).unwrap_err(),
            "Amount to withdraw 0.5 is greater than the holdings' total 0.3"
        );
    }

    fn contracts(issuer: bool) -> TokenStandardContracts<Cbtc> {
        let info = |name: &str| json!({"template_id": format!("pkg:{name}"), "contract_id": format!("00{name}"), "created_event_blob": "b"});
        let mut body = json!({"burn_mint_factory": info("factory"), "instrument_configuration": info("config")});
        if issuer {
            body["issuer_credential"] = info("issuer");
        }
        serde_json::from_value(body).unwrap()
    }

    #[test]
    fn the_burn_names_the_holdings_factory_and_reason_and_keeps_the_amount_decimal() {
        let account = account("0", json!(null));
        let holdings = [holding("00h1", REGISTRAR, "CBTC", "1")];
        let p = params(&account, &holdings, "0.00000001");
        let command = withdraw_exercise(&p, &contracts(true));
        assert_eq!(
            command.template_id,
            "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccount"
        );
        assert_eq!(command.contract_id, "00wa");
        assert_eq!(command.choice, "CBTCWithdrawAccount_Withdraw");
        let argument = &command.argument;
        assert_eq!(argument["amount"], json!("0.00000001"));
        assert_eq!(argument["tokens"], json!(["00h1"]));
        assert_eq!(argument["burnMintFactoryCid"], json!("00factory"));
        assert_eq!(argument["credentialCids"], json!(["00cred"]));
        assert_eq!(
            argument["extraArgs"]["meta"]["values"]["splice.lfdecentralizedtrust.org/reason"],
            json!("CBTC withdrawal")
        );
        let context = &argument["extraArgs"]["context"]["values"];
        assert_eq!(
            context["utility.digitalasset.com/instrument-configuration"],
            json!({"tag": "AV_ContractId", "value": "00config"})
        );
        assert_eq!(
            context["utility.digitalasset.com/issuer-credentials"],
            json!({"tag": "AV_List", "value": [{"tag": "AV_ContractId", "value": "00issuer"}]})
        );
        let disclosed: Vec<&str> = command
            .disclosed
            .iter()
            .map(|c| c.contract_id.as_str())
            .collect();
        assert_eq!(disclosed, vec!["00factory", "00config", "00issuer"]);
    }

    #[test]
    fn the_burn_without_an_issuer_credential_sends_an_empty_list() {
        let account = account("0", json!(null));
        let holdings = [holding("00h1", REGISTRAR, "CBTC", "1")];
        let p = params(&account, &holdings, "0.5");
        let command = withdraw_exercise(&p, &contracts(false));
        assert_eq!(
            command.argument["extraArgs"]["context"]["values"]["utility.digitalasset.com/issuer-credentials"],
            json!({"tag": "AV_List", "value": []})
        );
        assert_eq!(command.disclosed.len(), 2);
    }

    #[test]
    fn the_create_argument_uses_the_asset_destination_field() {
        let argument = create_withdraw_account_argument::<Cbtc>(
            "alice",
            "bcrt1qexample00000",
            &["00c".to_string()],
        );
        assert_eq!(
            argument,
            json!({"owner": "alice", "destinationBtcAddress": "bcrt1qexample00000", "credentialCids": ["00c"]})
        );
    }

    #[test]
    fn the_submitted_withdraw_parses_the_recreated_account() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": REGISTRAR,
            "destinationBtcAddress": "bcrt1qexample00000", "pendingBalance": "0.5", "limits": null,
        });
        let response = transaction_response(
            "tx-1",
            json!([created_event_value_with_blob(
                "f240:CBTC.WithdrawAccount:CBTCWithdrawAccount",
                "00new",
                args,
                "blob-new",
            )]),
        );
        let account = recreated_withdraw_account::<Cbtc>(&response).unwrap();
        assert_eq!(account.contract_id, "00new");
        assert_eq!(account.pending_balance, d("0.5"));
        assert_eq!(account.created_event_blob, "blob-new");
        assert_eq!(
            recreated_withdraw_account::<Cbtc>(&transaction_response("tx-2", json!(null)))
                .unwrap_err(),
            "No updated WithdrawAccount was found in the transaction"
        );
    }

    #[tokio::test]
    async fn create_withdraw_account_refuses_a_bad_destination_before_any_request() {
        let contract = |name: &str| json!({"template_id": format!("pkg:{name}"), "contract_id": format!("00{name}"), "created_event_blob": "blob"});
        let account_rules: AccountContractRuleSet<Cbtc> =
            serde_json::from_value(json!({"da_rules": contract("da"), "wa_rules": contract("wa")}))
                .unwrap();
        let error = match create_withdraw_account::<Cbtc>(CreateWithdrawAccountParams {
            ledger_host: "not-a-host".to_string(),
            party: "alice".to_string(),
            access_token: "token".to_string(),
            account_rules,
            destination_address: "b".repeat(13),
            credential_cids: vec!["00cred".to_string()],
        })
        .await
        {
            Ok(_) => panic!("a 13-character destination must be refused"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "destination address has 13 characters, expected 14 to 74"
        );
    }
}
