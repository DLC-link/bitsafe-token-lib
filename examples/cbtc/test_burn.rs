use bitsafe_token::credentials::ListCredentialsParams;
use bitsafe_token::tokens::cbtc;
use bitsafe_token::tokens::cbtc::redeem::{
    ListHoldingsParams, ListWithdrawAccountsParams, SubmitWithdrawParams,
};
/// Test burning CBTC using an existing withdraw account
///
/// This example demonstrates burning a small amount of CBTC using an existing
/// withdraw account instead of creating a new one.
///
/// Usage:
/// cargo run --example cbtc_test_burn
use keycloak::login::{PasswordParams, password, token_url};
use std::env;
#[path = "../shared.rs"]
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let params = PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };
    let login_response = password(params).await?;

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    // Fetch Minter credentials
    let credentials = bitsafe_token::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let minter_credential_cids = cbtc::minter_credential_cids(shared::network(), &credentials);

    if minter_credential_cids.is_empty() {
        return Err("No Minter credentials found. Run the credentials example first.".to_string());
    }

    let accounts = cbtc::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let my_accounts: Vec<_> = accounts.iter().filter(|a| a.owner == party_id).collect();

    if my_accounts.is_empty() {
        return Err(
            "No withdraw accounts found. Run 'cbtc_redeem_flow' example first.".to_string(),
        );
    }

    let withdraw_account = my_accounts[0];

    let holdings = cbtc::redeem::list_holdings(ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
        instrument_id: cbtc::instrument(shared::network()),
    })
    .await?;

    // list_holdings filtered the instrument. The owner test stays: it is a
    // separate condition, and this example burns only the caller's own tokens.
    let cbtc_holdings: Vec<_> = holdings.iter().filter(|h| h.owner == party_id).collect();

    if cbtc_holdings.is_empty() {
        return Err("No CBTC holdings found to burn".to_string());
    }

    let burn_amount = bitsafe_token::DamlDecimal::parse("0.0001").unwrap();

    let mut selected_holdings = Vec::new();
    let mut selected_total = bitsafe_token::DamlDecimal::ZERO;

    for holding in &cbtc_holdings {
        selected_holdings.push((*holding).clone());
        selected_total += holding.amount;
        if selected_total >= burn_amount {
            break;
        }
    }

    if selected_total < burn_amount {
        return Err(format!(
            "Insufficient balance. Have {}, need {}",
            selected_total, burn_amount
        ));
    }

    let updated_account = cbtc::redeem::submit_withdraw(
        shared::network(),
        SubmitWithdrawParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
            account: withdraw_account,
            amount: burn_amount,
            holdings: &selected_holdings,
            credential_cids: Some(minter_credential_cids),
        },
    )
    .await?;

    println!(
        "Burn successful! Pending balance: {} BTC",
        updated_account.pending_balance
    );

    Ok(())
}
