/// Example: Accept all pending transfers of the ASSET token
///
/// Run with: cargo run --example accept_transfers
///
/// Token Standard V2 needs no separate example. `accept::v2` re-exports V1's
/// `Params` and `AcceptAllParams` unchanged, so calling
/// `bitsafe_token::accept::v2::accept_all` with the same arguments is the whole
/// difference.
///
/// This example uses the `bitsafe_token::accept::accept_all` method to automatically
/// fetch and accept all pending TransferInstruction contracts of the ASSET token for your party.
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

    let decentralized_party_id = shared::registrar(shared::asset());

    let params = bitsafe_token::accept::AcceptAllParams {
        receiver_party: env::var("PARTY_ID").expect("PARTY_ID must be set"),
        instrument_id: shared::instrument(shared::asset()),
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id,
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    bitsafe_token::accept::accept_all(params).await?;

    Ok(())
}
