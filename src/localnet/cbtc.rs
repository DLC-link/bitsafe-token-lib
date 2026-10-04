//! The CBTC localnet suite: one test that runs the phases in order over one
//! fixture, because each phase uses the state the previous one left.

use serde_json::json;
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
        fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION},
        governance::run_action,
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

#[tokio::test]
#[ignore = "needs the localnet sandbox; see src/localnet/mod.rs"]
async fn localnet_cbtc() {
    let fixture = Fixture::new().await.expect("fixture");
    let rules = fixture
        .ledger
        .active(&fixture.registrar, GOVERNANCE_RULES)
        .await
        .expect("governance rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].created_event.contract_id, fixture.governance_rules);
    let instrument_configs = fixture
        .ledger
        .active(&fixture.registrar, INSTRUMENT_CONFIGURATION)
        .await
        .expect("instrument configurations");
    assert_eq!(instrument_configs.len(), 1);
    let rules = fixture.account_rules::<Cbtc>();
    assert_eq!(
        rules.da_rules.contract_id,
        fixture.deposit_rules.contract_id
    );
    assert!(!rules.da_rules.created_event_blob.is_empty());
    let contracts = fixture.token_standard_contracts::<Cbtc>();
    assert!(!contracts.burn_mint_factory.created_event_blob.is_empty());
    assert_eq!(fixture.instrument().id, "CBTC");

    let minter_cids = credentials_phase(&fixture).await;
    mint_phase(&fixture, &minter_cids).await;
    redeem_phase(&fixture, &minter_cids).await;
}

/// The credentials phase. The registrar offers the test user the CBTC Minter
/// claim, and the user accepts it. Returns the user's Minter credential cids.
async fn credentials_phase(fixture: &Fixture) -> Vec<String> {
    let registrar = fixture.registrar.as_str();

    // Before the offer, the user holds no Minter credential and no offer.
    let credentials = user_credentials(fixture).await;
    assert!(minter_credential_cids::<Cbtc>(registrar, &credentials).is_empty());
    let offers = user_offers(fixture).await;
    assert!(minter_credential_offers::<Cbtc>(registrar, &offers).is_empty());

    let offer = fixture
        .offer_minter_credential()
        .await
        .expect("offer the minter credential");
    let offers = user_offers(fixture).await;
    let pending = minter_credential_offers::<Cbtc>(registrar, &offers);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, offer);

    fixture
        .accept_offer(&offer)
        .await
        .expect("accept the minter credential");
    let credentials = user_credentials(fixture).await;
    let minter_cids = minter_credential_cids::<Cbtc>(registrar, &credentials);
    assert_eq!(minter_cids.len(), 1);

    // The public wrapper checks the network's registrar, which is not the
    // sandbox registrar.
    assert!(cbtc::minter_credential_cids(Network::Devnet, &credentials).is_empty());
    minter_cids
}

/// The mint phase. The user opens a deposit account with its Minter
/// credential, and the registrar completes a deposit of 1.0 into it through
/// governance. Returns the deposit account as the ledger lists it after the
/// mint.
async fn mint_phase(fixture: &Fixture, minter_cids: &[String]) -> DepositAccount<Cbtc> {
    assert!(user_deposit_accounts(fixture).await.is_empty());

    let account = create_deposit_account::<Cbtc>(CreateDepositAccountParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account_rules: fixture.account_rules(),
        credential_cids: minter_cids.to_vec(),
    })
    .await
    .expect("create the deposit account");
    assert_eq!(account.owner, fixture.user);
    assert_eq!(account.registrar, fixture.registrar);
    let accounts = user_deposit_accounts(fixture).await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].contract_id, account.contract_id);
    let found = find_deposit_account::<Cbtc>(deposit_params(fixture), &account.contract_id)
        .await
        .expect("find the deposit account");
    assert_eq!(found.contract_id, account.contract_id);

    // The extra args name the instrument configuration. The issuer
    // credential list stays empty, as in the burn, because the CBTC Daml
    // does not require one.
    let context = json!({
        "values": {
            "utility.digitalasset.com/instrument-configuration":
                {"tag": "AV_ContractId", "value": fixture.instrument_configuration.contract_id},
            "utility.digitalasset.com/issuer-credentials": {"tag": "AV_List", "value": []},
        }
    });
    let block = account.last_processed_block + 1;
    let response = run_action(
        fixture,
        DEPOSIT_COMPLETE_ACTION,
        json!({
            "governanceParty": fixture.registrar,
            "proposer": fixture.registrar,
            "description": "localnet deposit",
            "depositAccountCid": account.contract_id,
            "amount": "1.0",
            "bitcoinBlock": block.to_string(),
            "burnMintFactoryCid": fixture.allocation_factory.contract_id,
            "extraArgs": {"context": context, "meta": {"values": {}}},
            "credentialCids": minter_cids,
        }),
    )
    .await
    .expect("deposit completion");

    let holdings = user_holdings(fixture).await;
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].amount, DamlDecimal::parse("1.0").unwrap());

    // The mint replaces the account. The new contract keeps the old id as
    // its stable id and records the deposit's block.
    let minted =
        canton::created_by_suffix(&response, canton::template_suffix(Cbtc::DEPOSIT_ACCOUNT))
            .expect("the mint recreates the deposit account")
            .contract_id
            .clone();
    let account_after = find_deposit_account::<Cbtc>(deposit_params(fixture), &minted)
        .await
        .expect("find the deposit account after the mint");
    assert_eq!(account_after.account_id(), account.account_id());
    assert_eq!(account_after.last_processed_block, block);
    assert_eq!(user_deposit_accounts(fixture).await.len(), 1);
    account_after
}

/// The redeem phase. The user opens a withdraw account and burns the 1.0
/// holding into it. Then the registrar creates the withdraw request through
/// governance, which clears the pending balance.
async fn redeem_phase(fixture: &Fixture, minter_cids: &[String]) {
    assert!(user_withdraw_accounts(fixture).await.is_empty());
    let holdings = user_holdings(fixture).await;
    assert_eq!(holdings.len(), 1);
    assert_eq!(holdings[0].amount, DamlDecimal::parse("1.0").unwrap());
    assert!(user_withdraw_requests(fixture).await.is_empty());

    let account = create_withdraw_account::<Cbtc>(CreateWithdrawAccountParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account_rules: fixture.account_rules(),
        destination_address: DESTINATION.to_string(),
        credential_cids: minter_cids.to_vec(),
    })
    .await
    .expect("create the withdraw account");
    assert_eq!(account.owner, fixture.user);
    assert_eq!(account.registrar, fixture.registrar);
    assert_eq!(account.destination_address, DESTINATION);
    assert_eq!(account.pending_balance, DamlDecimal::ZERO);
    let accounts = user_withdraw_accounts(fixture).await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].contract_id, account.contract_id);
    let found = find_withdraw_account::<Cbtc>(withdraw_params(fixture), &account.contract_id)
        .await
        .expect("find the withdraw account");
    assert_eq!(found.contract_id, account.contract_id);

    // Both burns take the same parameters.
    let burn = || SubmitWithdrawParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        account: &account,
        amount: DamlDecimal::parse("1.0").unwrap(),
        holdings: &holdings,
        credential_cids: minter_cids.to_vec(),
    };

    // The public burn checks the network's registrar before it asks the
    // BitSafe API for anything, so it refuses the sandbox account.
    let error = match cbtc::redeem::submit_withdraw(Network::Devnet, burn()).await {
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

    let burned = submit_withdraw_with_contracts::<Cbtc>(
        &fixture.registrar,
        &fixture.token_standard_contracts(),
        burn(),
    )
    .await
    .expect("burn the holding");
    assert_eq!(burned.pending_balance, DamlDecimal::parse("1.0").unwrap());
    assert!(user_holdings(fixture).await.is_empty());

    // The action creates the request and recreates the account with a zero
    // pending balance.
    let response = run_action(
        fixture,
        WITHDRAW_PENDING_ACTION,
        json!({
            "governanceParty": fixture.registrar,
            "proposer": fixture.registrar,
            "description": "localnet withdraw",
            "withdrawAccountCid": burned.contract_id,
            "btcTxId": BTC_TX_ID,
        }),
    )
    .await
    .expect("withdraw request creation");
    let cleared =
        canton::created_by_suffix(&response, canton::template_suffix(Cbtc::WITHDRAW_ACCOUNT))
            .expect("the request recreates the withdraw account")
            .contract_id
            .clone();
    let account_after = find_withdraw_account::<Cbtc>(withdraw_params(fixture), &cleared)
        .await
        .expect("find the withdraw account after the request");
    assert_eq!(account_after.pending_balance, DamlDecimal::ZERO);
    assert_eq!(user_withdraw_accounts(fixture).await.len(), 1);

    let requests = user_withdraw_requests(fixture).await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].amount, DamlDecimal::parse("1.0").unwrap());
    assert_eq!(requests[0].destination_address, DESTINATION);
    assert_eq!(requests[0].btc_tx_id(), BTC_TX_ID);
}

/// The parameters that list the test user's deposit accounts.
fn deposit_params(fixture: &Fixture) -> ListDepositAccountsParams {
    ListDepositAccountsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    }
}

/// The CBTC deposit accounts the test user holds.
async fn user_deposit_accounts(fixture: &Fixture) -> Vec<DepositAccount<Cbtc>> {
    list_deposit_accounts::<Cbtc>(deposit_params(fixture))
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

/// The CBTC withdraw accounts the test user holds.
async fn user_withdraw_accounts(fixture: &Fixture) -> Vec<WithdrawAccount<Cbtc>> {
    list_withdraw_accounts::<Cbtc>(withdraw_params(fixture))
        .await
        .expect("list the user's withdraw accounts")
}

/// The CBTC withdraw requests the test user can see.
async fn user_withdraw_requests(fixture: &Fixture) -> Vec<WithdrawRequest<Cbtc>> {
    list_withdraw_requests::<Cbtc>(ListWithdrawRequestsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's withdraw requests")
}

/// The test user's holdings of the test registrar's CBTC.
async fn user_holdings(fixture: &Fixture) -> Vec<Holding> {
    list_holdings(ListHoldingsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
        instrument_id: fixture.instrument(),
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
