use bitsafe_token::credentials::ListCredentialsParams;
use bitsafe_token::tokens::beth;
use bitsafe_token::tokens::beth::mint::{
    CreateDepositAccountParams, ListDepositAccountsParams, U256,
};
/// BETH Minting Flow Example
///
/// This example builds the Ethereum transaction that mints BETH:
///
/// 1. Authenticate with Keycloak
/// 2. List the party's BETH deposit accounts
/// 3. Create a deposit account if the party has none
/// 4. Build the `depositETH` call for `DEPOSIT_AMOUNT_WEI`
///
/// The example sends nothing. Sign and send the printed transaction with
/// your own Ethereum wallet on the printed chain. The attestors mint BETH to
/// your party after the deposit's block is finalized.
///
/// The library does not read the bridge's live `depositLimits()` or
/// `paused()`. Check both with your Ethereum provider before you send.
///
/// To run this example:
/// 1. Copy .env.example to .env and fill in your values
/// 2. Accept a BETH Minter credential (run beth_credentials first)
/// 3. cargo run --example beth_mint_flow
use keycloak::login::{PasswordParams, password, token_url};
use std::env;
#[path = "../shared.rs"]
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== BETH Minting Flow Example ===\n");

    // Read the inputs first: a bad value stops the run before it logs in.
    let amount_text =
        shared::non_blank("DEPOSIT_AMOUNT_WEI").unwrap_or_else(|| "10000000000000000".to_string());
    let amount_wei: U256 = amount_text
        .parse()
        .map_err(|e| format!("DEPOSIT_AMOUNT_WEI is not a whole number of wei: {e}"))?;
    // The whole-unit check needs no ledger, so it runs before the login.
    beth::mint::wei_to_daml_decimal(amount_wei)?;
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

    println!("Step 2: Listing existing deposit accounts...");
    let accounts = beth::mint::list_deposit_accounts(ListDepositAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    println!("✓ Found {} existing deposit account(s)\n", accounts.len());

    // A BETH deposit id never changes, so the example reuses an account.
    let account = match accounts.into_iter().next() {
        Some(account) => {
            println!(
                "Step 3: Using the existing deposit account {}\n",
                account.contract_id
            );
            account
        }
        None => {
            println!("Step 3: Creating a deposit account...");
            let account_rules = beth::mint::get_account_contract_rules(network).await?;
            let credentials = bitsafe_token::credentials::list_credentials(ListCredentialsParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                access_token: access_token.clone(),
            })
            .await?;
            let credential_cids = beth::minter_credential_cids(network, &credentials);
            if credential_cids.is_empty() {
                return Err(
                    "No BETH Minter credentials found. Run beth_credentials first.".to_string(),
                );
            }
            let account = beth::mint::create_deposit_account(CreateDepositAccountParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                access_token: access_token.clone(),
                account_rules,
                credential_cids,
            })
            .await?;
            println!("✓ Created deposit account {}\n", account.contract_id);
            account
        }
    };

    println!("Step 4: Building the depositETH call...");
    let call = beth::mint::deposit_call(network, &account, amount_wei)?;
    println!("✓ Send this transaction with your own Ethereum wallet:");
    println!("  - Chain ID:   {}", call.chain_id);
    println!("  - To:         {}", call.to);
    println!("  - Value:      {} wei", call.value);
    println!("  - Data:       {}", call.data);
    println!("  - Deposit ID: {}", call.deposit_id);
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("The bridge refuses a deposit while it is paused or outside its live");
    println!("depositLimits(). Check both with your provider before you send.");
    println!("The attestors mint BETH after the deposit's block is finalized.");
    println!("Poll your BETH holdings on {network} to see the mint.");

    Ok(())
}
