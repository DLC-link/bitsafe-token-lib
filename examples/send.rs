/// Example: Send the ASSET token to another party
///
/// Run with: cargo run --example send
///
/// Make sure to set up your .env file with the required configuration.
///
/// Set ASSET (cbtc) and ENVIRONMENT (devnet, testnet or mainnet).
/// examples/README.md lists the other variables.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    // Authenticate
    println!("Authenticating...");
    let login_params = keycloak::login::PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    let auth = keycloak::login::password(login_params)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

    println!("Authenticated successfully!");

    // Set up transfer parameters
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let receiver_party = env::var("RECEIVER_PARTY_ID").unwrap_or_else(|_| {
        panic!(
            "RECEIVER_PARTY_ID must be set (the party to send {} to)",
            shared::asset().ticker
        )
    });
    let amount_str = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.1".to_string());
    let amount = bitsafe_token::DamlDecimal::parse(&amount_str).expect("Invalid TRANSFER_AMOUNT");

    println!("\nSending {} {}", amount, shared::asset().ticker);
    println!("From: {}", sender_party);
    println!("To: {}", receiver_party);

    // Create transfer
    let decentralized_party = shared::registrar(shared::asset());

    let transfer_params = bitsafe_token::transfer::Params {
        transfer: bitsafe_token::Transfer {
            sender: sender_party,
            receiver: receiver_party,
            amount,
            instrument_id: shared::instrument(shared::asset()),
            requested_at: chrono::Utc::now().to_rfc3339(),
            execute_before: chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(168))
                .unwrap()
                .to_rfc3339(),
            input_holding_cids: None, // Library will auto-select UTXOs
            meta: None,
        },
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        access_token: auth.access_token,
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id: decentralized_party,
    };

    // Submit transfer
    println!("\nSubmitting transfer...");
    let receipt = bitsafe_token::transfer::submit(transfer_params).await?;

    println!("✅ Transfer submitted successfully!");
    println!("   Update: {}", receipt.update_id);

    // The registry either creates an offer or settles the transfer outright.
    // Only the first leaves something for the receiver to do.
    match &receipt.outcome {
        bitsafe_token::transfer::TransferOutcome::Pending {
            transfer_instruction_cid,
        } => {
            println!("   Offer:  {}", transfer_instruction_cid);
            println!("\nThe receiver must accept the offer for the transfer to complete.");
        }
        bitsafe_token::transfer::TransferOutcome::Completed {
            receiver_holding_cids,
        } => {
            println!("\nThe transfer settled on submission, so there is nothing to accept.");
            println!("   Receiver holdings:");
            for cid in receiver_holding_cids {
                println!("     - {}", cid);
            }
        }
    }

    if !receipt.sender_change_cids.is_empty() {
        println!("   Your change holdings:");
        for cid in &receipt.sender_change_cids {
            println!("     - {}", cid);
        }
    }

    Ok(())
}
