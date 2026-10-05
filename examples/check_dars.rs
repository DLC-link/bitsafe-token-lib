/// Example: Check the ASSET token's DAR packages on participant
///
/// Verifies that the participant node holds every DAR package the ASSET
/// token needs, by scanning the asset's DAR folders under dars/ and comparing
/// them against the participant.
///
/// Run with: cargo run --example check_dars
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST
/// - ASSET (cbtc or beth)
use std::env;
use std::path::Path;
use std::process;
mod shared;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let asset = shared::asset();

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
        .unwrap_or_else(|e| {
            eprintln!("Authentication failed: {}", e);
            process::exit(1);
        });

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");

    println!(
        "Checking the {} DAR packages on participant...",
        asset.ticker
    );
    println!("  Ledger host: {}", ledger_host);
    println!();

    let result = bitsafe_token::check_dars(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        asset.dar_dirs,
        ledger_host,
        auth.access_token,
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("DAR check failed: {}", e);
        process::exit(1);
    });

    // Print results
    println!(
        "Found {}/{} expected packages",
        result.found.len(),
        result.total_expected
    );
    println!();

    match result.status {
        bitsafe_token::dar_check::DarCheckStatus::Pass => {
            println!("PASS: All required DAR packages are present.");
        }
        bitsafe_token::dar_check::DarCheckStatus::Fail => {
            println!(
                "FAIL: {} of {} packages are missing:",
                result.missing.len(),
                result.total_expected
            );
            println!();
            for package in &result.missing {
                println!(
                    "  {} v{} ({})",
                    package.name, package.version, package.package_id
                );
            }
            println!();
            println!("Note: Missing packages may not yet be required for your environment.");
            println!(
                "This repo may include DARs ahead of what is deployed on Canton Network mainnet."
            );
            println!();
            println!("To verify which versions are required for your environment:");
            println!(
                "  Splice DARs:  https://github.com/hyperledger-labs/splice/tree/main/daml/dars"
            );
            println!("                (select the tag matching your environment release)");
            println!("  Utility DARs: https://docs.digitalasset.com/utilities/releases/index.html");
            println!();
            println!(
                "To upload missing DARs to your participant: {} {}",
                concat!(env!("CARGO_MANIFEST_DIR"), "/dars/upload_dars.sh"),
                asset.ticker.to_ascii_lowercase()
            );
            process::exit(1);
        }
    }
}
