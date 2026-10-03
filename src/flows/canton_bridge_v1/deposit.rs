//! The deposit account flows of the family.

use serde_json::{Value, json};

use crate::{
    flows::canton_bridge_v1::{CantonBridgeV1, created_contract_id, models::DepositAccount},
    kits::{
        bitsafe_api::{self, AccountContractRuleSet},
        canton::{self, Exercise},
    },
};

/// The parameters for listing a party's deposit accounts.
pub struct ListDepositAccountsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// The parameters for creating a deposit account.
pub struct CreateDepositAccountParams<A> {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    /// The rules from `get_account_contract_rules`, disclosed to the ledger.
    pub account_rules: AccountContractRuleSet<A>,
    /// The party's Minter credential cids, from `minter_credential_cids`.
    pub credential_cids: Vec<String>,
}

/// Fetches the registrar-signed rules contracts from the BitSafe API.
pub(crate) async fn get_account_contract_rules<A: CantonBridgeV1>(
    api_url: &str,
) -> Result<AccountContractRuleSet<A>, String> {
    bitsafe_api::get_json(
        api_url,
        &format!("{}/v1/account-contract-rules", A::API_PATH),
    )
    .await
}

/// Lists the party's live deposit accounts of this asset.
pub(crate) async fn list_deposit_accounts<A: CantonBridgeV1>(
    params: ListDepositAccountsParams,
) -> Result<Vec<DepositAccount<A>>, String> {
    canton::list_by_template(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        A::DEPOSIT_ACCOUNT,
    )
    .await?
    .iter()
    .map(DepositAccount::from_active_contract)
    .collect()
}

/// Lists the party's deposit accounts and picks one by contract id.
pub(crate) async fn find_deposit_account<A: CantonBridgeV1>(
    params: ListDepositAccountsParams,
    contract_id: &str,
) -> Result<DepositAccount<A>, String> {
    list_deposit_accounts(params)
        .await?
        .into_iter()
        .find(|account| account.contract_id == contract_id)
        .ok_or_else(|| format!("Deposit account with contract ID {} not found", contract_id))
}

/// The choice argument of `CreateDepositAccount`.
pub(crate) fn create_deposit_account_argument(party: &str, credential_cids: &[String]) -> Value {
    json!({ "owner": party, "credentialCids": credential_cids })
}

/// Creates a deposit account and returns it as the ledger now lists it.
pub(crate) async fn create_deposit_account<A: CantonBridgeV1>(
    params: CreateDepositAccountParams<A>,
) -> Result<DepositAccount<A>, String> {
    let rules = &params.account_rules.da_rules;
    let response = canton::exercise(
        &params.ledger_host,
        &params.party,
        &params.access_token,
        Exercise {
            template_id: A::DEPOSIT_ACCOUNT_RULES,
            contract_id: &rules.contract_id,
            choice: A::CREATE_DEPOSIT_ACCOUNT_CHOICE,
            argument: create_deposit_account_argument(&params.party, &params.credential_cids),
            disclosed: vec![rules.disclosed()],
        },
    )
    .await?;
    let contract_id = created_contract_id(&response, A::DEPOSIT_ACCOUNT, "DepositAccount")?;
    let accounts = list_deposit_accounts::<A>(ListDepositAccountsParams {
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
                "Created DepositAccount {} not found in active contracts",
                contract_id
            )
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_create_argument_names_the_owner_and_the_credentials() {
        let argument = create_deposit_account_argument("alice::1220", &["00cred".to_string()]);
        assert_eq!(
            argument,
            json!({"owner": "alice::1220", "credentialCids": ["00cred"]})
        );
    }
}
