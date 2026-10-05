use bitsafe_token::credentials::ListCredentialsParams;
use bitsafe_token::tokens::cbtc;
use bitsafe_token::tokens::cbtc::redeem::{
    CreateWithdrawAccountParams, ListHoldingsParams, ListWithdrawAccountsParams,
    SubmitWithdrawParams,
};
/// CBTC Redeeming (Withdrawal) Flow Example
///
/// This example demonstrates the complete flow of submitting a CBTC withdrawal:
///
/// 1. Authenticate with Keycloak
/// 2. Get account rules from the attestor network
/// 3. Find the withdraw account for DESTINATION_BTC_ADDRESS, or create one
/// 4. List existing CBTC holdings
/// 5. Submit withdrawal (burn CBTC and increase pending balance)
/// 6. Verify the withdrawal was submitted successfully
///
/// DESTINATION_BTC_ADDRESS sets the Bitcoin address that receives the BTC.
/// It has no default. WITHDRAW_AMOUNT sets the amount to burn. The default
/// is 0.001.
///
/// Note: WithdrawRequests are NOT created atomically with the withdrawal submission.
/// The attestor network will create WithdrawRequests later. Use the separate
/// `cbtc_check_withdraw_requests` example to monitor for processed withdrawals.
///
/// To run this example:
/// 1. Make sure you have .env configured with your credentials and
///    DESTINATION_BTC_ADDRESS
/// 2. Make sure you have CBTC holdings (run cbtc_mint_flow first)
/// 3. cargo run --example cbtc_redeem_flow
use keycloak::login::{PasswordParams, password, token_url};
use std::env;
#[path = "../shared.rs"]
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Redeeming (Withdrawal) Flow Example ===\n");

    // Read the destination first: a missing value stops the run before it
    // logs in. There is no default address, because a default would send
    // the payout to an address the person did not choose.
    let destination_btc_address = shared::non_blank("DESTINATION_BTC_ADDRESS")
        .ok_or("Set DESTINATION_BTC_ADDRESS to the Bitcoin address that receives the BTC")?;

    // Step 1: Authenticate with Keycloak
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

    // Common parameters
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    // Step 2: List existing withdraw accounts
    println!("Step 2: Listing existing withdraw accounts...");
    let accounts = cbtc::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    println!("✓ Found {} existing withdraw account(s)", accounts.len());
    for account in &accounts {
        println!("  - Contract ID: {}", account.contract_id);
        println!("    Owner: {}", account.owner);
        println!(
            "    Destination BTC Address: {}",
            account.destination_address
        );
    }
    println!();

    // Step 3: Check CBTC holdings
    println!("Step 3: Checking CBTC holdings...");
    let holdings = cbtc::redeem::list_holdings(ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
        instrument_id: cbtc::instrument(shared::network()),
    })
    .await?;

    // list_holdings already filtered by instrument, so every holding here is
    // one this flow can redeem.
    let total_cbtc: bitsafe_token::DamlDecimal = holdings.iter().map(|h| h.amount).sum();

    println!("✓ Found {} CBTC holding(s)", holdings.len());
    println!("  Total CBTC balance: {} BTC", total_cbtc);
    for holding in &holdings {
        println!(
            "    - {} BTC (CID: {})",
            holding.amount, holding.contract_id
        );
    }
    println!();

    if holdings.is_empty() {
        println!("⚠ You don't have any CBTC holdings to redeem.");
        println!("  Run 'cbtc_mint_flow' example first to mint some CBTC.");
        return Ok(());
    }

    // Step 4: Get account rules from Bitsafe API
    println!("Step 4: Getting account contract rules from Bitsafe API...");
    let account_rules = cbtc::mint::get_account_contract_rules(shared::network()).await?;
    println!("✓ Retrieved account rules:");
    println!(
        "  - WithdrawAccountRules CID: {}",
        account_rules.wa_rules.contract_id
    );
    println!();

    // Step 4b: Fetch Minter credentials
    println!("Step 4b: Fetching Minter credentials...");
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
    println!(
        "  Found {} Minter credential(s)\n",
        minter_credential_cids.len()
    );

    // Step 5: Burn only into an account that pays out to
    // DESTINATION_BTC_ADDRESS. The comparison is exact, because a legacy
    // Bitcoin address is case-sensitive.
    let existing = accounts
        .into_iter()
        .find(|account| account.destination_address == destination_btc_address);
    let withdraw_account = match existing {
        Some(account) => {
            println!(
                "Step 5: Using the existing withdraw account {} for {}\n",
                account.contract_id, account.destination_address
            );
            account
        }
        None => {
            println!("Step 5: Creating a new withdraw account...");
            println!("  Destination BTC address: {}", destination_btc_address);

            let withdraw_account =
                cbtc::redeem::create_withdraw_account(CreateWithdrawAccountParams {
                    ledger_host: ledger_host.clone(),
                    party: party_id.clone(),
                    access_token: access_token.clone(),
                    account_rules: account_rules.clone(),
                    destination_address: destination_btc_address.clone(),
                    credential_cids: minter_credential_cids.clone(),
                })
                .await?;

            println!("✓ Withdraw account created successfully!");
            println!("  - Contract ID: {}", withdraw_account.contract_id);
            println!("  - Owner: {}", withdraw_account.owner);
            println!(
                "  - Destination BTC Address: {}",
                withdraw_account.destination_address
            );
            println!();
            withdraw_account
        }
    };

    // Step 6: Submit withdrawal (burn CBTC)
    let withdraw_amount =
        shared::non_blank("WITHDRAW_AMOUNT").unwrap_or_else(|| "0.001".to_string());
    let withdraw_amount_decimal = bitsafe_token::DamlDecimal::parse(&withdraw_amount)
        .map_err(|e| format!("WITHDRAW_AMOUNT is not a number: {e}"))?;

    if total_cbtc < withdraw_amount_decimal {
        println!(
            "⚠ Insufficient CBTC balance. You have {} but trying to withdraw {}",
            total_cbtc, withdraw_amount
        );
        return Ok(());
    }

    println!("Step 6: Submitting withdrawal (burning CBTC)...");
    println!("  Amount to withdraw: {} BTC", withdraw_amount);

    // Select holdings to burn - for simplicity, just use the first holding with enough balance
    // or combine multiple holdings
    let mut selected_holdings = Vec::new();
    let mut selected_total = bitsafe_token::DamlDecimal::ZERO;

    for holding in &holdings {
        selected_holdings.push(holding.clone());
        selected_total += holding.amount;

        if selected_total >= withdraw_amount_decimal {
            break;
        }
    }

    println!(
        "  Using {} holding(s) totaling {} BTC",
        selected_holdings.len(),
        selected_total
    );

    let updated_account = cbtc::redeem::submit_withdraw(
        shared::network(),
        SubmitWithdrawParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
            account: &withdraw_account,
            amount: withdraw_amount_decimal,
            holdings: &selected_holdings,
            credential_cids: minter_credential_cids,
        },
    )
    .await?;

    println!("✓ Withdrawal submitted successfully!");
    println!(
        "  - Updated Account Contract ID: {}",
        updated_account.contract_id
    );
    println!(
        "  - Pending Balance: {} BTC",
        updated_account.pending_balance
    );
    println!("  - Destination: {}", updated_account.destination_address);
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Summary:");
    println!(
        "  • Your withdraw account contract ID: {}",
        updated_account.contract_id
    );
    println!(
        "  • Pending balance: {} BTC",
        updated_account.pending_balance
    );
    println!(
        "  • BTC will be sent to: {}",
        updated_account.destination_address
    );
    println!();
    println!("Important: WithdrawRequests are NOT created atomically with this call.");
    println!("The attestor network will process your pending balance and create a");
    println!("WithdrawRequest later. Use 'cbtc_check_withdraw_requests' to monitor:");
    println!("  cargo run --example cbtc_check_withdraw_requests");

    Ok(())
}
