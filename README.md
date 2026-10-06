# bitsafe-token

A Rust client library for BitSafe's bridged assets on Canton Network: CBTC
(Bitcoin) and BETH (Ether). One API shape serves both assets. The library
also re-exports the Canton Token Standard operations of
[canton-lib](https://github.com/DLC-link/canton-lib), so you can hold, send
and receive both assets.

**Status: 0.1.0.** The API can change in any 0.x release. The crate is not on
crates.io; you add it as a git dependency. It needs Rust 1.94 or newer
(edition 2024).

## Install

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
bitsafe-token = { git = "https://github.com/DLC-link/bitsafe-token-lib", tag = "v0.1.0" }
# Login. Or use your own OpenID Connect client and pass its access token.
keycloak = { git = "https://github.com/DLC-link/canton-lib", tag = "v0.10.0" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

If you add other `canton-lib` crates, pin the same tag, `v0.10.0`. Two
different pins make Cargo build two copies of the shared types, and then the
types do not match.

## Before you start

You need these, for the network you use:

1. **A Canton participant node** with the JSON ledger API, such as the
   participant of your Canton Network validator.
2. **A party** on that participant. A party is your identity on the ledger,
   such as `your-party::1220...`.
3. **A Keycloak (or other OpenID Connect) user** that can act as your party.
4. **The Digital Asset Registry Utility** on your validator, for holdings and
   transfers.
5. **The DAR packages** on your participant. See
   [`dars/README.md`](dars/README.md); `check_dars` tells you what is
   missing.
6. **A Minter credential,** to mint or burn only. The asset's registrar
   offers it to your party; ask BitSafe for it. You accept the offer once.
   Holdings and transfers need no credential.

## Concepts

| Term | Meaning |
|---|---|
| Registrar | The party that issues an asset on one network. Each asset module knows its registrar for each network |
| Minter credential | A credential from the registrar with the claim `hasCBTCRole` or `hasBETHRole` = `Minter`. The mint and burn calls need it |
| Deposit account | A contract that ties your party to a deposit: a BTC address for CBTC, a deposit id for BETH |
| Withdraw account | A contract that holds your payout address and your pending (burned) balance |
| Withdraw request | The payout record. The attestors create it after a burn and archive it after the payout |
| Holding | One unit of an asset that your party owns, like a UTXO |

## Quick start: CBTC

Mint: create a deposit account and get the BTC address to fund. Burn: burn
CBTC into a withdraw account. The attestors then pay out BTC.

```rust
use bitsafe_token::{
    DamlDecimal, Network,
    credentials::{ListCredentialsParams, list_credentials},
    tokens::cbtc::{self, mint, redeem},
};
use keycloak::login::{PasswordParams, password, token_url};

#[tokio::main]
async fn main() -> Result<(), String> {
    let network = Network::Devnet;
    let ledger_host = "https://participant.example.com/api/json-api".to_string();
    let party = "your-party::1220...".to_string();

    // Log in as a user that can act as `party`.
    let access_token = password(PasswordParams {
        client_id: "your-client-id".into(),
        username: "your-username".into(),
        password: "your-password".into(),
        url: token_url("https://keycloak.example.com", "your-realm"),
    })
    .await?
    .access_token;

    // The registrar's Minter credential, which you accepted once.
    let credentials = list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    let credential_cids = cbtc::minter_credential_cids(network, &credentials);

    // Mint: create a deposit account and get the BTC address to fund.
    let rules = mint::get_account_contract_rules(network).await?;
    let account = mint::create_deposit_account(mint::CreateDepositAccountParams {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        access_token: access_token.clone(),
        account_rules: rules.clone(),
        credential_cids: credential_cids.clone(),
    })
    .await?;
    let btc_address = mint::get_bitcoin_address(network, &account).await?;
    println!("Send BTC to {btc_address}");

    // Burn: create a withdraw account, then burn 0.001 CBTC into it.
    let withdraw_account = redeem::create_withdraw_account(redeem::CreateWithdrawAccountParams {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        access_token: access_token.clone(),
        account_rules: rules,
        destination_address: "your-btc-address".into(),
        credential_cids: credential_cids.clone(),
    })
    .await?;
    let holdings = redeem::list_holdings(redeem::ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        access_token: access_token.clone(),
        instrument_id: cbtc::instrument(network),
    })
    .await?;
    let withdraw_account = redeem::submit_withdraw(
        network,
        redeem::SubmitWithdrawParams {
            ledger_host: ledger_host.clone(),
            party: party.clone(),
            access_token: access_token.clone(),
            account: &withdraw_account,
            amount: DamlDecimal::parse("0.001").map_err(|e| e.to_string())?,
            holdings: &holdings,
            credential_cids,
        },
    )
    .await?;
    println!("Pending: {}", withdraw_account.pending_balance);

    // Later: the attestors create a withdraw request and pay out.
    let requests = redeem::list_withdraw_requests(redeem::ListWithdrawRequestsParams {
        ledger_host,
        party,
        access_token,
    })
    .await?;
    println!("{} withdraw request(s)", requests.len());
    Ok(())
}
```

## Quick start: BETH

BETH uses the same calls under `tokens::beth`. Only the deposit step
differs: `deposit_call` builds the Ethereum transaction that mints BETH. The
library holds no keys and sends nothing. You sign and send the transaction
with your own wallet, on the chain in `call.chain_id`.

```rust
use bitsafe_token::{
    Network,
    tokens::beth::mint::{self, U256},
};

// Log in and find the Minter credential as in the CBTC example, with
// `beth::minter_credential_cids`.
async fn mint_beth(
    network: Network,
    ledger_host: String,
    party: String,
    access_token: String,
    credential_cids: Vec<String>,
) -> Result<(), String> {
    let rules = mint::get_account_contract_rules(network).await?;
    let account = mint::create_deposit_account(mint::CreateDepositAccountParams {
        ledger_host,
        party,
        access_token,
        account_rules: rules,
        credential_cids,
    })
    .await?;

    // 0.01 ETH, in wei.
    let call = mint::deposit_call(network, &account, U256::from(10_000_000_000_000_000_u64))?;
    println!("chain {} to {} value {} data {}", call.chain_id, call.to, call.value, call.data);
    Ok(())
}
```

**Units.** The two directions use different units:

- `deposit_call` takes **wei** as a `U256`. 1 ETH is 10^18 wei, and the
  amount must be a whole multiple of 100000000 wei.
- `redeem::submit_withdraw` takes **ETH** as a `DamlDecimal`, such as `0.01`,
  the same as CBTC takes BTC.
- `beth::mint::wei_to_daml_decimal` and `daml_decimal_to_wei` convert.

**Chains.** Devnet and testnet BETH run on Sepolia (chain 11155111), so you
need Sepolia ETH. Mainnet BETH runs on Ethereum mainnet (chain 1). The
library does not read the bridge's live `depositLimits()` or `paused()`;
check both with your Ethereum provider before you send.

## Hold and send

The re-exported `TokenClient` reads balances and sends either asset. The
asset module gives it the right configuration for each network:

```rust
use bitsafe_token::{
    DamlDecimal, KeycloakConfig, Network, SendParams, TokenClient, TokenStandardVersion,
    tokens::cbtc,
};
use keycloak::login::token_url;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut client = TokenClient::connect(cbtc::client_config(
        Network::Mainnet,
        "https://participant.example.com/api/json-api".into(),
        "your-party::1220...".into(),
        KeycloakConfig {
            client_id: "your-client-id".into(),
            username: "your-username".into(),
            password: "your-password".into(),
            url: token_url("https://keycloak.example.com", "your-realm"),
        },
        TokenStandardVersion::V2,
    ))
    .await?;

    println!("balance {} CBTC", client.balance().await?);
    client
        .send(SendParams {
            receiver: "receiver-party::1220...".into(),
            amount: DamlDecimal::parse("0.01").map_err(|e| e.to_string())?,
            reference: None,
            execute_before: None,
            input_holding_cids: None,
        })
        .await?;
    Ok(())
}
```

On mainnet this moves real funds. Use `token_url` for the Keycloak URL; the
older `password_url` builds a path that current Keycloak servers do not
serve. The Token Standard has two entry points: V1 names a party as a string,
and V2 names an `Account`. Both move the same holdings, and the examples use
V2 where both exist.

## Networks

`Network` is a plain enum: `Devnet`, `Testnet`, `Mainnet`. Each asset module
maps a `Network` to its own registrar, so you pass only the network.

| Network | Utility registry | BitSafe API | BETH chain |
| --- | --- | --- | --- |
| devnet | https://api.utilities.digitalasset-dev.com | https://api.devnet.bitsafe.finance | Sepolia, 11155111 |
| testnet | https://api.utilities.digitalasset-staging.com | https://api.testnet.bitsafe.finance | Sepolia, 11155111 |
| mainnet | https://api.utilities.digitalasset.com | https://api.mainnet.bitsafe.finance | Ethereum, 1 |

## What the library does not do

- **It returns errors as text.** Every function returns `Result<_, String>`.
- **It does not wait for a mint.** Poll your holdings to see the minted
  amount.
- **It lists only active withdraw requests.** The attestors archive a request
  after the payout, so a paid request no longer appears.
- **It does not check the network of a BTC address.** Use an address of the
  Bitcoin network that your Canton network pays out on.
- **It reads no Ethereum state.** It reads no bridge limits, no pause state
  and no deposit events.

## Examples

[`examples/README.md`](examples/README.md) lists 30 runnable examples, for
both assets: credentials, mint, burn, transfers, allocations and batch
distribution. Copy [`.env.example`](.env.example) to `.env`, fill it in, and
run `cargo run --example <name>`.

## Contributing

Run the gates before you open a pull request:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

`tokens::<asset>` is the whole public, asset-facing API. `kits` and `flows`
are crate-private building blocks. `tokens` may use `flows` and `kits`,
`flows` may use `kits`, and nothing imports upward; review checks this.

The localnet suite runs the CBTC and BETH flows against a local Canton
sandbox. It needs Docker:

```sh
docker compose -f localnet/docker-compose.yml up -d --wait
cargo test --lib localnet -- --ignored --nocapture --test-threads=1
docker compose -f localnet/docker-compose.yml down -v
```

The first start pulls the image and bootstraps the sandbox, which takes
several minutes. Set `LOCALNET_LEDGER_HOST` to use a ledger other than
`http://localhost:7575`.

## License

MIT. See [`LICENSE`](LICENSE).
