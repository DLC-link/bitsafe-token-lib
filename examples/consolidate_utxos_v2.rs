/// Example: Check and consolidate UTXOs, Token Standard V2
///
/// Run with: cargo run --example consolidate_utxos_v2
///
/// The V2 counterpart of `consolidate_utxos`. The only difference is the
/// account: V1 takes the party as a string, and V2 takes a
/// `bitsafe_token::Account`. `Account::basic` builds the unlabelled account every
/// party owns, with no provider and an empty id. A party can also hold tokens
/// under a labelled account, and this example does not reach those.
///
/// Set ASSET (cbtc or beth) and ENVIRONMENT (devnet, testnet or mainnet).
/// examples/README.md lists the other variables.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("Authenticating...");
    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let threshold: usize = env::var("CONSOLIDATION_THRESHOLD")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("CONSOLIDATION_THRESHOLD must be a valid number");

    println!("\n🔄 Checking UTXO consolidation (V2) for party:");
    println!("   Party: {}", party);
    println!("   Threshold: {} UTXOs\n", threshold);

    let decentralized_party_id = shared::registrar(shared::asset());

    let result = bitsafe_token::consolidate::v2::check_and_consolidate(
        bitsafe_token::consolidate::v2::CheckConsolidateParams {
            account: bitsafe_token::Account::basic(party),
            instrument_id: shared::instrument(shared::asset()),
            threshold,
            ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
            access_token: auth.access_token,
            registry_url: shared::resolve_registry_url(),
            decentralized_party_id,
        },
    )
    .await?;

    println!();
    if result.consolidated {
        println!("✅ Consolidation complete!");
        println!("   Before: {} UTXOs", result.utxos_before);
        println!("   After:  {} UTXO(s)", result.utxos_after);
        println!("\n   Resulting holding CIDs:");
        for cid in &result.holding_cids {
            let short = if cid.len() > 16 {
                format!("{}...{}", &cid[..8], &cid[cid.len() - 8..])
            } else {
                cid.clone()
            };
            println!("     - {}", short);
        }
    } else {
        println!("✅ No consolidation needed");
        println!("   Current UTXO count: {}", result.utxos_before);
        println!("   Threshold: {}", threshold);
    }

    Ok(())
}
