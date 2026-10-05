//! The steps that every asset of the `canton_bridge_v1` family runs the same
//! way. Each asset's suite composes them and adds what only that asset has:
//! its governance action fields, its public wrappers and its extra checks.

use serde_json::{Value, json};
use token::holding::Holding;

use crate::{
    DamlDecimal, Network,
    credentials::{
        CredentialOffer, ListCredentialOffersParams, ListCredentialsParams, UserCredential,
        list_credential_offers, list_credentials,
    },
    flows::canton_bridge_v1::{
        CantonBridgeV1,
        credentials::{minter_credential_cids, minter_credential_offers},
        deposit::{
            CreateDepositAccountParams, ListDepositAccountsParams, create_deposit_account,
            find_deposit_account, list_deposit_accounts,
        },
        models::{DepositAccount, WithdrawAccount, WithdrawRequest},
        withdraw::{
            CreateWithdrawAccountParams, ListWithdrawAccountsParams, ListWithdrawRequestsParams,
            SubmitWithdrawParams, create_withdraw_account, find_withdraw_account,
            list_withdraw_accounts, list_withdraw_requests, submit_withdraw_with_contracts,
        },
    },
    kits::canton::{self, ListHoldingsParams, list_holdings},
    localnet::{
        fixture::{AssetStack, Fixture},
        governance::run_action,
    },
};

/// The amount every suite mints and then burns.
pub(crate) const AMOUNT: &str = "1.0";

/// The credentials phase. The registrar offers the test user the Minter
/// claim of `A`, and the user accepts it. `public_cids` is the asset's
/// public `minter_credential_cids`, which must find nothing, because the
/// issuer is not a production registrar. Returns the user's Minter
/// credential cids for `A`.
pub(crate) async fn credentials_phase<A: CantonBridgeV1>(
    fixture: &Fixture,
    public_cids: fn(Network, &[UserCredential]) -> Vec<String>,
) -> Vec<String> {
    let registrar = fixture.registrar.as_str();

    // Before the offer, the user holds no Minter credential and no offer of A.
    let credentials = user_credentials(fixture).await;
    assert!(minter_credential_cids::<A>(registrar, &credentials).is_empty());
    let offers = user_offers(fixture).await;
    assert!(minter_credential_offers::<A>(registrar, &offers).is_empty());

    let offer = fixture
        .offer_minter_credential::<A>()
        .await
        .expect("offer the minter credential");
    let offers = user_offers(fixture).await;
    let pending = minter_credential_offers::<A>(registrar, &offers);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, offer);

    fixture
        .accept_offer(&offer)
        .await
        .expect("accept the minter credential");
    let credentials = user_credentials(fixture).await;
    let minter_cids = minter_credential_cids::<A>(registrar, &credentials);
    assert_eq!(minter_cids.len(), 1);

    // The public wrapper checks the network's registrar, which is not the
    // sandbox registrar.
    assert!(public_cids(Network::Devnet, &credentials).is_empty());
    minter_cids
}

/// The first mint steps. The user has no deposit account of `A`, opens one
/// with its Minter credential, and finds it by listing and by contract id.
pub(crate) async fn open_deposit_account<A: CantonBridgeV1>(
    fixture: &Fixture,
    stack: &AssetStack<A>,
    minter_cids: &[String],
) -> DepositAccount<A> {
    assert!(user_deposit_accounts::<A>(fixture).await.is_empty());

    let account = create_deposit_account::<A>(CreateDepositAccountParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account_rules: stack.account_rules(),
        credential_cids: minter_cids.to_vec(),
    })
    .await
    .expect("create the deposit account");
    assert_eq!(account.owner, fixture.user);
    assert_eq!(account.registrar, fixture.registrar);
    let accounts = user_deposit_accounts::<A>(fixture).await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].contract_id, account.contract_id);
    let found = find_deposit_account::<A>(deposit_params(fixture), &account.contract_id)
        .await
        .expect("find the deposit account");
    assert_eq!(found.contract_id, account.contract_id);
    account
}

/// The registrar completes a deposit of `AMOUNT` into `account` through the
/// governance action `action`, whose source-chain block field is
/// `block_field`. Returns the deposit account as the ledger lists it after
/// the mint.
pub(crate) async fn complete_deposit<A: CantonBridgeV1>(
    fixture: &Fixture,
    stack: &AssetStack<A>,
    account: &DepositAccount<A>,
    minter_cids: &[String],
    action: &str,
    block_field: &str,
) -> DepositAccount<A> {
    // The extra args name the instrument configuration. The issuer
    // credential list stays empty, as in the burn, because neither Daml
    // package requires one.
    let context = json!({
        "values": {
            "utility.digitalasset.com/instrument-configuration":
                {"tag": "AV_ContractId", "value": stack.instrument_configuration.contract_id},
            "utility.digitalasset.com/issuer-credentials": {"tag": "AV_List", "value": []},
        }
    });
    let block = account.last_processed_block + 1;
    let mut arguments = json!({
        "governanceParty": fixture.registrar,
        "proposer": fixture.registrar,
        "description": "localnet deposit",
        "depositAccountCid": account.contract_id,
        "amount": AMOUNT,
        "burnMintFactoryCid": fixture.allocation_factory.contract_id,
        "extraArgs": {"context": context, "meta": {"values": {}}},
        "credentialCids": minter_cids,
    });
    arguments[block_field] = json!(block.to_string());
    let response = run_action(fixture, action, arguments)
        .await
        .expect("deposit completion");

    let holdings = user_holdings(fixture, stack).await;
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].amount, DamlDecimal::parse(AMOUNT).unwrap());

    // The mint replaces the account. The new contract keeps the old id as
    // its stable id and records the deposit's block.
    let minted = canton::created_by_suffix(&response, canton::template_suffix(A::DEPOSIT_ACCOUNT))
        .expect("the mint recreates the deposit account")
        .contract_id
        .clone();
    let account_after = find_deposit_account::<A>(deposit_params(fixture), &minted)
        .await
        .expect("find the deposit account after the mint");
    assert_eq!(account_after.account_id(), account.account_id());
    assert_eq!(account_after.last_processed_block, block);
    assert_eq!(user_deposit_accounts::<A>(fixture).await.len(), 1);
    account_after
}

/// The first redeem steps. The user holds the minted `AMOUNT`, has no
/// withdraw account and no withdraw request of `A`, opens a withdraw account
/// for `destination`, and finds it. Returns the account and the holdings.
pub(crate) async fn open_withdraw_account<A: CantonBridgeV1>(
    fixture: &Fixture,
    stack: &AssetStack<A>,
    minter_cids: &[String],
    destination: &str,
) -> (WithdrawAccount<A>, Vec<Holding>) {
    assert!(user_withdraw_accounts::<A>(fixture).await.is_empty());
    let holdings = user_holdings(fixture, stack).await;
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].amount, DamlDecimal::parse(AMOUNT).unwrap());
    assert!(user_withdraw_requests::<A>(fixture).await.is_empty());

    let account = create_withdraw_account::<A>(CreateWithdrawAccountParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account_rules: stack.account_rules(),
        destination_address: destination.to_string(),
        credential_cids: minter_cids.to_vec(),
    })
    .await
    .expect("create the withdraw account");
    assert_eq!(account.owner, fixture.user);
    assert_eq!(account.registrar, fixture.registrar);
    assert_eq!(account.destination_address, destination);
    assert_eq!(account.pending_balance, DamlDecimal::ZERO);
    let accounts = user_withdraw_accounts::<A>(fixture).await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].contract_id, account.contract_id);
    let found = find_withdraw_account::<A>(withdraw_params(fixture), &account.contract_id)
        .await
        .expect("find the withdraw account");
    assert_eq!(found.contract_id, account.contract_id);
    (account, holdings)
}

/// The parameters that burn `AMOUNT` from `holdings` into `account`.
pub(crate) fn burn_params<'a, A>(
    fixture: &Fixture,
    account: &'a WithdrawAccount<A>,
    holdings: &'a [Holding],
    minter_cids: &[String],
) -> SubmitWithdrawParams<'a, A> {
    SubmitWithdrawParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account,
        amount: DamlDecimal::parse(AMOUNT).unwrap(),
        holdings,
        credential_cids: minter_cids.to_vec(),
    }
}

/// The burn, with the token standard contracts read from the ledger. Returns
/// the account with its pending balance.
pub(crate) async fn burn<A: CantonBridgeV1>(
    fixture: &Fixture,
    stack: &AssetStack<A>,
    params: SubmitWithdrawParams<'_, A>,
) -> WithdrawAccount<A> {
    let burned = submit_withdraw_with_contracts::<A>(
        &fixture.registrar,
        &stack.token_standard_contracts(),
        params,
    )
    .await
    .expect("burn the holding");
    assert_eq!(burned.pending_balance, DamlDecimal::parse(AMOUNT).unwrap());
    assert!(user_holdings(fixture, stack).await.is_empty());
    burned
}

/// The registrar creates the withdraw request for `burned` through the
/// governance action `action`. `field` is the action field that only this
/// asset has. Returns the one withdraw request the user then sees.
pub(crate) async fn create_withdraw_request<A: CantonBridgeV1>(
    fixture: &Fixture,
    burned: &WithdrawAccount<A>,
    action: &str,
    field: (&str, Value),
) -> WithdrawRequest<A> {
    let mut arguments = json!({
        "governanceParty": fixture.registrar,
        "proposer": fixture.registrar,
        "description": "localnet withdraw",
        "withdrawAccountCid": burned.contract_id,
    });
    arguments[field.0] = field.1;
    // The action creates the request and recreates the account with a zero
    // pending balance.
    let response = run_action(fixture, action, arguments)
        .await
        .expect("withdraw request creation");
    let cleared =
        canton::created_by_suffix(&response, canton::template_suffix(A::WITHDRAW_ACCOUNT))
            .expect("the request recreates the withdraw account")
            .contract_id
            .clone();
    let account_after = find_withdraw_account::<A>(withdraw_params(fixture), &cleared)
        .await
        .expect("find the withdraw account after the request");
    assert_eq!(account_after.pending_balance, DamlDecimal::ZERO);
    assert_eq!(user_withdraw_accounts::<A>(fixture).await.len(), 1);

    let mut requests = user_withdraw_requests::<A>(fixture).await;
    assert_eq!(requests.len(), 1);
    let request = requests.remove(0);
    assert_eq!(request.amount, DamlDecimal::parse(AMOUNT).unwrap());
    assert_eq!(request.destination_address, burned.destination_address);
    request
}

/// The parameters that list the test user's deposit accounts.
fn deposit_params(fixture: &Fixture) -> ListDepositAccountsParams {
    ListDepositAccountsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    }
}

/// The deposit accounts of `A` that the test user holds.
async fn user_deposit_accounts<A: CantonBridgeV1>(fixture: &Fixture) -> Vec<DepositAccount<A>> {
    list_deposit_accounts::<A>(deposit_params(fixture))
        .await
        .expect("list the user's deposit accounts")
}

/// The parameters that list the test user's withdraw accounts.
fn withdraw_params(fixture: &Fixture) -> ListWithdrawAccountsParams {
    ListWithdrawAccountsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    }
}

/// The withdraw accounts of `A` that the test user holds.
async fn user_withdraw_accounts<A: CantonBridgeV1>(fixture: &Fixture) -> Vec<WithdrawAccount<A>> {
    list_withdraw_accounts::<A>(withdraw_params(fixture))
        .await
        .expect("list the user's withdraw accounts")
}

/// The withdraw requests of `A` that the test user can see.
async fn user_withdraw_requests<A: CantonBridgeV1>(fixture: &Fixture) -> Vec<WithdrawRequest<A>> {
    list_withdraw_requests::<A>(ListWithdrawRequestsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's withdraw requests")
}

/// The test user's holdings of the asset's test instrument.
async fn user_holdings<A: CantonBridgeV1>(
    fixture: &Fixture,
    stack: &AssetStack<A>,
) -> Vec<Holding> {
    list_holdings(ListHoldingsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        instrument_id: stack.instrument(),
    })
    .await
    .expect("list the user's holdings")
}

/// The credentials the test user holds.
async fn user_credentials(fixture: &Fixture) -> Vec<UserCredential> {
    list_credentials(ListCredentialsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credentials")
}

/// The credential offers the test user holds.
async fn user_offers(fixture: &Fixture) -> Vec<CredentialOffer> {
    list_credential_offers(ListCredentialOffersParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credential offers")
}
