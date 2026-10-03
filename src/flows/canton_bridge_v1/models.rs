//! The account and request models of the family. Each model carries its
//! asset as a type parameter, so a CBTC account cannot reach a BETH function.
//! Each keeps its raw create argument for a field that the typed model does not read.

use std::marker::PhantomData;

use ledger::models::JsActiveContract;
use serde_json::Value;

use crate::{
    DamlDecimal,
    flows::canton_bridge_v1::CantonBridgeV1,
    kits::canton::{
        Limits, check_limits, create_args, optional_limits, optional_str, required_str,
    },
};

/// A deposit account. `account_id()` is the id the attestors key on.
#[derive(Debug, Clone)]
pub struct DepositAccount<A> {
    pub contract_id: String,
    pub template_id: String,
    /// The account's stable id: the original contract id. `None` on a fresh
    /// account, whose contract id is then the id.
    pub id: Option<String>,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    /// The last source-chain block the attestors processed for this account.
    pub last_processed_block: i64,
    pub limits: Option<Limits>,
    /// The raw create argument, for a field only one asset has.
    pub create_argument: Value,
    _asset: PhantomData<A>,
}

impl<A> DepositAccount<A> {
    /// The id the attestors look the account up by: `id`, else the contract
    /// id. A deposit made to any other id is never credited.
    pub fn account_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.contract_id)
    }

    /// Checks an amount against the account's limits, so a caller can refuse
    /// a deposit before it sends one. An account without limits accepts any
    /// amount. This method does not refuse zero or a negative amount; the
    /// caller checks that the amount is above zero.
    ///
    /// # Errors
    ///
    /// Fails with the bound that the amount breaks, for example
    /// `Deposit amount 0.0005 is below minimum 0.001`.
    pub fn check_amount(&self, amount: DamlDecimal) -> Result<(), String> {
        check_limits("Deposit", amount, &self.limits)
    }
}

impl<A: CantonBridgeV1> DepositAccount<A> {
    pub(crate) fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let args = create_args(contract)?;
        let id = optional_str(args, "id");
        let owner = required_str(args, "owner")?;
        let operator = required_str(args, "operator")?;
        let registrar = required_str(args, "registrar")?;
        let block_field = A::LAST_PROCESSED_BLOCK_FIELD;
        let last_processed_block = args
            .get(block_field)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| format!("Missing or invalid '{}' field", block_field))?;
        let limits = optional_limits(args)?;
        Ok(Self {
            contract_id: contract.created_event.contract_id.clone(),
            template_id: contract.created_event.template_id.clone(),
            id,
            owner,
            operator,
            registrar,
            last_processed_block,
            limits,
            create_argument: Value::Object(args.clone()),
            _asset: PhantomData,
        })
    }
}

/// A withdraw account: where a burn is recorded and the payout goes.
#[derive(Debug, Clone)]
pub struct WithdrawAccount<A> {
    pub contract_id: String,
    pub template_id: String,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    pub destination_address: String,
    /// The amount burned and not yet paid out. A new burn needs zero.
    pub pending_balance: DamlDecimal,
    pub created_event_blob: String,
    pub limits: Option<Limits>,
    /// The raw create argument, for a field only one asset has.
    pub create_argument: Value,
    _asset: PhantomData<A>,
}

impl<A: CantonBridgeV1> WithdrawAccount<A> {
    pub(crate) fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let args = create_args(contract)?;
        let owner = required_str(args, "owner")?;
        let operator = required_str(args, "operator")?;
        let registrar = required_str(args, "registrar")?;
        let destination_address = required_str(args, A::DESTINATION_ADDRESS_FIELD)?;
        let pending_balance = DamlDecimal::parse(
            args.get("pendingBalance")
                .and_then(|v| v.as_str())
                .unwrap_or("0"),
        )
        .map_err(|e| format!("Invalid 'pendingBalance' field: {}", e))?;
        let limits = optional_limits(args)?;
        Ok(Self {
            contract_id: contract.created_event.contract_id.clone(),
            template_id: contract.created_event.template_id.clone(),
            owner,
            operator,
            registrar,
            destination_address,
            pending_balance,
            created_event_blob: contract
                .created_event
                .created_event_blob
                .clone()
                .unwrap_or_default(),
            limits,
            create_argument: Value::Object(args.clone()),
            _asset: PhantomData,
        })
    }
}

impl<A> WithdrawAccount<A> {
    /// Checks an amount against the account's limits, so a caller can refuse
    /// a burn before it submits one. An account without limits accepts any
    /// amount. This method does not refuse zero or a negative amount;
    /// `submit_withdraw` does.
    ///
    /// # Errors
    ///
    /// Fails with the bound that the amount breaks, for example
    /// `Withdraw amount 2 exceeds maximum 1`.
    pub fn check_amount(&self, amount: DamlDecimal) -> Result<(), String> {
        check_limits("Withdraw", amount, &self.limits)
    }
}

/// A payout record the registrar creates after a burn.
#[derive(Debug, Clone)]
pub struct WithdrawRequest<A: CantonBridgeV1> {
    pub contract_id: String,
    pub template_id: String,
    pub owner: String,
    pub registrar: String,
    pub amount: DamlDecimal,
    pub destination_address: String,
    /// The withdraw account the request came from, when the ledger records it.
    pub source_account_id: Option<String>,
    /// The fields that only this asset has. A field that the Daml template
    /// requires is required here too.
    pub details: A::WithdrawRequestDetails,
    /// The raw create argument, for a field that the typed model does not read.
    pub create_argument: Value,
    _asset: PhantomData<A>,
}

impl<A: CantonBridgeV1> WithdrawRequest<A> {
    pub(crate) fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let args = create_args(contract)?;
        let owner = required_str(args, "owner")?;
        let registrar = required_str(args, "registrar")?;
        let amount = DamlDecimal::parse(&required_str(args, "amount")?)
            .map_err(|e| format!("Invalid 'amount' field: {}", e))?;
        let destination_address = required_str(args, A::DESTINATION_ADDRESS_FIELD)?;
        let source_account_id = optional_str(args, "sourceAccountId");
        let details = A::parse_withdraw_request_details(args)?;
        Ok(Self {
            contract_id: contract.created_event.contract_id.clone(),
            template_id: contract.created_event.template_id.clone(),
            owner,
            registrar,
            amount,
            destination_address,
            source_account_id,
            details,
            create_argument: Value::Object(args.clone()),
            _asset: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{test_fixtures::active_contract_of, tokens::cbtc::Cbtc};

    const DA: &str = "f240:CBTC.DepositAccount:CBTCDepositAccount";
    const WA: &str = "f240:CBTC.WithdrawAccount:CBTCWithdrawAccount";
    const WR: &str = "f240:CBTC.WithdrawRequest:CBTCWithdrawRequest";

    fn d(s: &str) -> DamlDecimal {
        DamlDecimal::parse(s).unwrap()
    }

    fn deposit_args(id: Value) -> Value {
        json!({
            "id": id, "owner": "alice", "operator": "op", "registrar": "cbtc-network::1220",
            "lastProcessedBitcoinBlock": "812345", "limits": {"minAmount": "0.001", "maxAmount": null},
        })
    }

    #[test]
    fn a_deposit_account_parses_with_its_chain_field_and_template() {
        let contract = active_contract_of(DA, "00da", deposit_args(json!("00orig")));
        let account = DepositAccount::<Cbtc>::from_active_contract(&contract).unwrap();
        assert_eq!(account.contract_id, "00da");
        assert_eq!(account.template_id, DA);
        assert_eq!(account.registrar, "cbtc-network::1220");
        assert_eq!(account.last_processed_block, 812345);
        assert_eq!(
            account.limits.unwrap().min_amount,
            Some(DamlDecimal::parse("0.001").unwrap())
        );
        assert_eq!(account.create_argument["owner"], "alice");
    }

    #[test]
    fn account_id_is_the_stable_id_else_the_contract_id() {
        let with_id = active_contract_of(DA, "00current", deposit_args(json!("00orig")));
        assert_eq!(
            DepositAccount::<Cbtc>::from_active_contract(&with_id)
                .unwrap()
                .account_id(),
            "00orig"
        );
        let legacy = active_contract_of(DA, "00current", deposit_args(json!(null)));
        assert_eq!(
            DepositAccount::<Cbtc>::from_active_contract(&legacy)
                .unwrap()
                .account_id(),
            "00current"
        );
    }

    #[test]
    fn a_deposit_account_without_its_block_field_fails_by_name() {
        let mut args = deposit_args(json!(null));
        args.as_object_mut()
            .unwrap()
            .remove("lastProcessedBitcoinBlock");
        let error =
            DepositAccount::<Cbtc>::from_active_contract(&active_contract_of(DA, "00x", args))
                .unwrap_err();
        assert_eq!(
            error,
            "Missing or invalid 'lastProcessedBitcoinBlock' field"
        );
    }

    #[test]
    fn a_withdraw_account_parses_its_destination_balance_and_blob() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": "cbtc-network::1220",
            "destinationBtcAddress": "bcrt1qexample00000", "pendingBalance": "0.5", "limits": null,
        });
        let account =
            WithdrawAccount::<Cbtc>::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert_eq!(account.destination_address, "bcrt1qexample00000");
        assert_eq!(account.pending_balance, DamlDecimal::parse("0.5").unwrap());
        assert_eq!(account.created_event_blob, "blob-of-00wa");
        assert_eq!(account.template_id, WA);
        assert!(account.limits.is_none());
    }

    #[test]
    fn a_withdraw_account_without_a_pending_balance_reads_zero() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": "r", "destinationBtcAddress": "bcrt1qexample00000",
        });
        let account =
            WithdrawAccount::<Cbtc>::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert_eq!(account.pending_balance, DamlDecimal::ZERO);
    }

    fn request_args() -> Value {
        json!({
            "owner": "alice", "registrar": "cbtc-network::1220", "amount": "0.25",
            "destinationBtcAddress": "bcrt1qexample00000", "btcTxId": "abc123", "sourceAccountId": "00wa",
        })
    }

    #[test]
    fn a_withdraw_request_parses_with_its_source_account() {
        let request = WithdrawRequest::<Cbtc>::from_active_contract(&active_contract_of(
            WR,
            "00wr",
            request_args(),
        ))
        .unwrap();
        assert_eq!(request.amount, DamlDecimal::parse("0.25").unwrap());
        assert_eq!(request.destination_address, "bcrt1qexample00000");
        assert_eq!(request.source_account_id.as_deref(), Some("00wa"));
        assert_eq!(request.template_id, WR);
    }

    #[test]
    fn a_cbtc_withdraw_request_carries_its_btc_tx_id() {
        let request = WithdrawRequest::<Cbtc>::from_active_contract(&active_contract_of(
            WR,
            "00wr",
            request_args(),
        ))
        .unwrap();
        assert_eq!(request.details.btc_tx_id, "abc123");
        assert_eq!(request.btc_tx_id(), "abc123");
    }

    #[test]
    fn a_cbtc_withdraw_request_without_btc_tx_id_does_not_parse() {
        let mut args = request_args();
        args.as_object_mut().unwrap().remove("btcTxId");
        let error =
            WithdrawRequest::<Cbtc>::from_active_contract(&active_contract_of(WR, "00wr", args))
                .unwrap_err();
        assert_eq!(error, "Missing 'btcTxId' field");
    }

    #[test]
    fn check_amount_tests_a_deposit_against_the_account_limits() {
        let account = DepositAccount::<Cbtc>::from_active_contract(&active_contract_of(
            DA,
            "00da",
            deposit_args(json!(null)),
        ))
        .unwrap();
        assert!(account.check_amount(d("0.001")).is_ok());
        assert_eq!(
            account.check_amount(d("0.0005")).unwrap_err(),
            "Deposit amount 0.0005 is below minimum 0.001"
        );
    }

    #[test]
    fn check_amount_tests_a_withdraw_against_the_account_limits() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": "r", "destinationBtcAddress": "bcrt1qexample00000",
            "pendingBalance": "0", "limits": {"minAmount": null, "maxAmount": "1"},
        });
        let account =
            WithdrawAccount::<Cbtc>::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert!(account.check_amount(d("1")).is_ok());
        assert_eq!(
            account.check_amount(d("2")).unwrap_err(),
            "Withdraw amount 2 exceeds maximum 1"
        );
    }

    #[test]
    fn check_amount_accepts_any_amount_on_an_account_without_limits() {
        let args = json!({
            "owner": "alice", "operator": "op", "registrar": "r", "destinationBtcAddress": "bcrt1qexample00000",
            "pendingBalance": "0", "limits": null,
        });
        let account =
            WithdrawAccount::<Cbtc>::from_active_contract(&active_contract_of(WA, "00wa", args))
                .unwrap();
        assert!(account.check_amount(d("1000000")).is_ok());
    }
}
