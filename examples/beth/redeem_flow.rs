use bitsafe_token::credentials::ListCredentialsParams;
use bitsafe_token::tokens::beth;
use bitsafe_token::tokens::beth::redeem::{
    CreateWithdrawAccountParams, ListHoldingsParams, ListWithdrawAccountsParams,
    ListWithdrawRequestsParams, SubmitWithdrawParams,
};
/// BETH Redeeming (Withdrawal) Flow Example
///
/// This example burns BETH so that the registrar pays out ETH:
///
/// 1. Authenticate with Keycloak
/// 2. List the party's BETH withdraw accounts
/// 3. List the party's BETH holdings
/// 4. Find the withdraw account for `DESTINATION_ETH_ADDRESS`, or create one
/// 5. Burn `WITHDRAW_AMOUNT` BETH into the account
/// 6. List the party's withdraw requests
///
/// The registrar creates a withdraw request after the burn and archives it
/// when it pays the ETH. A live BETH request carries no transaction id.
///
/// To run this example:
/// 1. Copy .env.example to .env, fill in your values and set DESTINATION_ETH_ADDRESS
/// 2. Make sure you have BETH holdings (run beth_mint_flow first)
/// 3. cargo run --example beth_redeem_flow
use keycloak::login::{PasswordParams, password, token_url};
use std::env;
#[path = "../shared.rs"]
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== BETH Redeeming (Withdrawal) Flow Example ===\n");

    // Read the inputs first: a missing value or a bad amount stops the run
    // before it logs in. The library checks the address when it creates the
    // withdraw account. There is no default address, because any 0x address is
    // a real address and a default would send the payout to someone else.
    let destination = shared::non_blank("DESTINATION_ETH_ADDRESS")
        .ok_or("Set DESTINATION_ETH_ADDRESS to the 0x address that receives the ETH")?;
    let withdraw_amount =
        shared::non_blank("WITHDRAW_AMOUNT").unwrap_or_else(|| "0.01".to_string());
    let amount = bitsafe_token::DamlDecimal::parse(&withdraw_amount)
        .map_err(|e| format!("WITHDRAW_AMOUNT is not a number: {e}"))?;
    let network = shared::network();

    println!("Step 1: Authenticating with Keycloak...");
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
    println!("✓ Authenticated successfully\n");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    println!("Step 2: Listing existing withdraw accounts...");
    let accounts = beth::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    println!("✓ Found {} existing withdraw account(s)\n", accounts.len());

    println!("Step 3: Checking BETH holdings...");
    let holdings = beth::redeem::list_holdings(ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
        instrument_id: beth::instrument(network),
    })
    .await?;
    let total: bitsafe_token::DamlDecimal = holdings.iter().map(|h| h.amount).sum();
    println!(
        "✓ Found {} BETH holding(s), total {} ETH\n",
        holdings.len(),
        total
    );
    if total < amount {
        println!("⚠ The holdings total {total}, less than WITHDRAW_AMOUNT {amount}.");
        println!("  Run beth_mint_flow first to mint BETH.");
        return Ok(());
    }

    let credentials = bitsafe_token::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    let credential_cids = beth::minter_credential_cids(network, &credentials);
    if credential_cids.is_empty() {
        return Err("No BETH Minter credentials found. Run beth_credentials first.".to_string());
    }

    // Burn only into an account that pays out to DESTINATION_ETH_ADDRESS. The
    // comparison ignores ASCII case, because the checksum changes only the case.
    let existing = accounts.into_iter().find(|account| {
        account
            .destination_address
            .eq_ignore_ascii_case(&destination)
    });
    let account = match existing {
        Some(account) => {
            println!(
                "Step 4: Using the existing withdraw account {} for {}\n",
                account.contract_id, account.destination_address
            );
            account
        }
        None => {
            println!("Step 4: Creating a withdraw account for {destination}...");
            let account_rules = beth::redeem::get_account_contract_rules(network).await?;
            let account = beth::redeem::create_withdraw_account(CreateWithdrawAccountParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                access_token: access_token.clone(),
                account_rules,
                destination_address: destination.clone(),
                credential_cids: credential_cids.clone(),
            })
            .await?;
            println!("✓ Created withdraw account {}\n", account.contract_id);
            account
        }
    };

    // Take holdings until their sum covers the amount.
    let mut selected = Vec::new();
    let mut selected_total = bitsafe_token::DamlDecimal::ZERO;
    for holding in &holdings {
        if selected_total >= amount {
            break;
        }
        selected.push(holding.clone());
        selected_total += holding.amount;
    }

    println!(
        "Step 5: Burning {amount} BETH from {} holding(s)...",
        selected.len()
    );
    let updated = beth::redeem::submit_withdraw(
        network,
        SubmitWithdrawParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
            account: &account,
            amount,
            holdings: &selected,
            credential_cids,
        },
    )
    .await?;
    println!(
        "✓ Burn submitted. Pending balance: {} ETH\n",
        updated.pending_balance
    );

    println!("Step 6: Listing withdraw requests...");
    let requests = beth::redeem::list_withdraw_requests(ListWithdrawRequestsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    println!("✓ Found {} withdraw request(s)", requests.len());
    for request in &requests {
        println!(
            "  - {} ETH to {} (contract {})",
            request.amount, request.destination_address, request.contract_id
        );
    }
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("The registrar creates the withdraw request after the burn, so the");
    println!("request can appear only on a later run. The registrar archives the");
    println!("request when it pays the ETH; the bridge's Withdraw event then holds");
    println!("the Ethereum transaction.");

    Ok(())
}
