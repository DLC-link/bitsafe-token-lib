//! The BETH phases of the localnet suite: credentials, mint and redeem, in
//! that order, because each phase uses the state the previous one left.

use serde_json::json;

use crate::{
    Network,
    kits::evm::evm_asset_bridge::encode_deposit_eth,
    localnet::{
        fixture::Fixture,
        phases::{
            AMOUNT, burn, burn_params, complete_deposit, create_withdraw_request,
            credentials_phase, open_deposit_account, open_withdraw_account,
        },
    },
    tokens::beth::{
        self, Beth, BethNetworkConfig,
        mint::{U256, deposit_call_for},
    },
};

const DEPOSIT_COMPLETE_ACTION: &str =
    "#beth-governance-actions-v1:BETH.GovernanceActions.DepositComplete:DepositCompleteAction";
const WITHDRAW_PENDING_ACTION: &str =
    "#beth-governance-actions-v1:BETH.GovernanceActions.WithdrawPending:WithdrawPendingAction";
/// The Ethereum address the test user withdraws to, with its EIP-55 checksum.
const DESTINATION: &str = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";

/// Runs the BETH phases over the shared fixture.
pub(crate) async fn run(fixture: &Fixture) {
    let stack = fixture.asset_stack::<Beth>().await.expect("the BETH stack");
    assert!(!stack.deposit_rules.created_event_blob.is_empty());
    assert!(!stack.instrument_configuration.created_event_blob.is_empty());
    assert_eq!(stack.instrument().id, "BETH");

    let minter_cids = credentials_phase::<Beth>(fixture, beth::minter_credential_cids).await;

    let account = open_deposit_account(fixture, &stack, &minter_cids).await;
    // The deposit call with the sandbox registrar. The config holds a
    // `'static` registrar, so the suite leaks one string per run.
    let sandbox = BethNetworkConfig {
        registrar: Box::leak(fixture.registrar.clone().into_boxed_str()),
        bridge: beth::bridge(Network::Devnet),
    };
    // 0.01 ETH. The suite sends no ETH: this amount only exercises the
    // encoder. The governance mint below credits AMOUNT on its own.
    let amount_wei = U256::from(10_000_000_000_000_000_u64);
    let call = deposit_call_for(&sandbox, &account, amount_wei).expect("the deposit call");
    assert_eq!(call.deposit_id, account.account_id());
    assert_eq!(call.data, encode_deposit_eth(account.account_id()));
    assert_eq!(call.to, sandbox.bridge.proxy);
    assert_eq!(call.value, amount_wei);
    // The public wrapper resolves the devnet registrar, which is not the
    // sandbox registrar.
    assert_eq!(
        beth::mint::deposit_call(Network::Devnet, &account, amount_wei).unwrap_err(),
        format!(
            "network mismatch: account registrar {}, expected {}",
            fixture.registrar,
            beth::registrar(Network::Devnet)
        )
    );

    let minted = complete_deposit(
        fixture,
        &stack,
        &account,
        &minter_cids,
        DEPOSIT_COMPLETE_ACTION,
        "ethereumBlock",
    )
    .await;
    // The mint recreates the account, but the attestors keep matching
    // deposits on the stable id, so the calldata must not change.
    let call_after = deposit_call_for(&sandbox, &minted, amount_wei).expect("the deposit call");
    assert_eq!(call_after.data, call.data);

    let (account, holdings) =
        open_withdraw_account(fixture, &stack, &minter_cids, DESTINATION).await;
    // The public burn checks the network's registrar before it asks the
    // BitSafe API for anything, so it refuses the sandbox account.
    let error = match beth::redeem::submit_withdraw(
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
            beth::registrar(Network::Devnet)
        )
    );
    let burned = burn(
        fixture,
        &stack,
        burn_params(fixture, &account, &holdings, &minter_cids),
    )
    .await;
    // The BETH action names the amount, which must equal the pending
    // balance. A live BETH request carries no transaction id.
    let request = create_withdraw_request(
        fixture,
        &burned,
        WITHDRAW_PENDING_ACTION,
        ("amount", json!(AMOUNT)),
    )
    .await;
    assert_eq!(request.owner, fixture.user);
    assert_eq!(request.registrar, fixture.registrar);
}
