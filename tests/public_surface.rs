//! Every name `cbtc-lib` re-exported from `canton-lib` is reachable here under
//! the same path. A missing name is a compile error, which is the test.

#[allow(unused_imports)]
use bitsafe_token::{
    DamlDecimal, DistributeParams, InstrumentId, KeycloakConfig, Meta, SendParams, SplitParams,
    TokenClient, TokenClientConfig, TokenStandardVersion, Transfer, accept, active_contracts,
    allocation, batch, cancel_offers, consolidate, credentials, dar_check, distribute, holding,
    reject, split, transfer, types, utils,
};

#[test]
fn the_re_exports_compile() {
    let _: fn() -> bitsafe_token::types::v2::Transfer = || unreachable!();
    let _: Option<bitsafe_token::Account> = None;
}

/// The whole CBTC API is reachable under `tokens::cbtc`, with no generic
/// parameter and no crate-private path. A missing name is a compile error.
#[test]
fn the_cbtc_api_compiles_at_its_public_paths() {
    use bitsafe_token::tokens::cbtc::{
        self, Cbtc, INFO, TICKER, client_config, instrument,
        mint::{
            AccountContractRuleSet, ContractInfo, CreateDepositAccountParams, DepositAccount,
            DepositAccountStatus, Limits, ListDepositAccountsParams, create_deposit_account,
            find_deposit_account, get_account_contract_rules, get_bitcoin_address,
            get_deposit_account_status, list_deposit_accounts,
        },
        minter_credential_cids, minter_credential_offers,
        redeem::{
            CreateWithdrawAccountParams, ListHoldingsParams, ListWithdrawAccountsParams,
            ListWithdrawRequestsParams, SubmitWithdrawParams, WithdrawAccount, WithdrawRequest,
            create_withdraw_account, find_withdraw_account, list_holdings, list_withdraw_accounts,
            list_withdraw_requests, submit_withdraw,
        },
    };
    let _ = (
        Cbtc,
        INFO,
        TICKER,
        cbtc::registrar(bitsafe_token::Network::Devnet),
    );
    let _ = (
        client_config,
        instrument,
        minter_credential_cids,
        minter_credential_offers,
    );
    let _ = (
        create_deposit_account,
        find_deposit_account,
        get_account_contract_rules,
    );
    let _ = (
        get_bitcoin_address,
        get_deposit_account_status,
        list_deposit_accounts,
    );
    let _ = (
        create_withdraw_account,
        find_withdraw_account,
        list_holdings,
    );
    let _ = (
        list_withdraw_accounts,
        list_withdraw_requests,
        submit_withdraw,
    );
    let _: Option<(
        DepositAccount,
        DepositAccountStatus,
        AccountContractRuleSet,
        ContractInfo,
        Limits,
    )> = None;
    let _: Option<(ListDepositAccountsParams, CreateDepositAccountParams)> = None;
    let _: Option<(WithdrawAccount, WithdrawRequest, ListWithdrawAccountsParams)> = None;
    let _: Option<(
        CreateWithdrawAccountParams,
        ListHoldingsParams,
        ListWithdrawRequestsParams,
    )> = None;
    let _: Option<SubmitWithdrawParams<'static>> = None;
}
