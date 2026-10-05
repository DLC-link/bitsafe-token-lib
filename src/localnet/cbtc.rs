//! The CBTC phases of the localnet suite: credentials, mint and redeem, in
//! that order, because each phase uses the state the previous one left.

use serde_json::json;

use crate::{
    Network,
    localnet::{
        fixture::Fixture,
        phases::{
            burn, burn_params, complete_deposit, create_withdraw_request, credentials_phase,
            open_deposit_account, open_withdraw_account,
        },
    },
    tokens::cbtc::{self, Cbtc},
};

const DEPOSIT_COMPLETE_ACTION: &str =
    "#cbtc-governance-actions-v1:CBTC.GovernanceActions.DepositComplete:DepositCompleteAction";
const WITHDRAW_PENDING_ACTION: &str =
    "#cbtc-governance-actions-v1:CBTC.GovernanceActions.WithdrawPending:WithdrawPendingAction";
/// The Bitcoin address the test user withdraws to.
const DESTINATION: &str = "bcrt1qlocalnet000000";
/// The Bitcoin transaction id the registrar records on the withdraw request.
const BTC_TX_ID: &str = "localnet-btc-tx";

/// Runs the CBTC phases over the shared fixture.
pub(crate) async fn run(fixture: &Fixture) {
    let stack = fixture.asset_stack::<Cbtc>().await.expect("the CBTC stack");
    assert!(!stack.deposit_rules.created_event_blob.is_empty());
    assert!(!stack.instrument_configuration.created_event_blob.is_empty());
    assert_eq!(stack.instrument().id, "CBTC");

    let minter_cids = credentials_phase::<Cbtc>(fixture, cbtc::minter_credential_cids).await;

    let account = open_deposit_account(fixture, &stack, &minter_cids).await;
    complete_deposit(
        fixture,
        &stack,
        &account,
        &minter_cids,
        DEPOSIT_COMPLETE_ACTION,
        "bitcoinBlock",
    )
    .await;

    let (account, holdings) =
        open_withdraw_account(fixture, &stack, &minter_cids, DESTINATION).await;
    // The public burn checks the network's registrar before it asks the
    // BitSafe API for anything, so it refuses the sandbox account.
    let error = match cbtc::redeem::submit_withdraw(
        Network::Devnet,
        burn_params(fixture, &account, &holdings, &minter_cids),
    )
    .await
    {
        Ok(_) => panic!("the devnet burn must refuse a sandbox account"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        format!(
            "network mismatch: account registrar {}, expected {}",
            fixture.registrar,
            cbtc::registrar(Network::Devnet)
        )
    );
    let burned = burn(
        fixture,
        &stack,
        burn_params(fixture, &account, &holdings, &minter_cids),
    )
    .await;
    let request = create_withdraw_request(
        fixture,
        &burned,
        WITHDRAW_PENDING_ACTION,
        ("btcTxId", json!(BTC_TX_ID)),
    )
    .await;
    assert_eq!(request.btc_tx_id(), BTC_TX_ID);
}
