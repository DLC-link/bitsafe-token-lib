# Examples

This directory contains example programs demonstrating how to use the `bitsafe-token` library.

## Setup

1. Copy `.env.example` to `.env` in the project root:

   ```bash
   cp .env.example .env
   ```

2. Fill in your configuration values in `.env`

## Running Examples

Run every example from the project root with `cargo run --example <name>`.

### Two groups of examples

**Token Standard examples** live in `examples/`. They work for any asset, and
`ASSET` in your `.env` picks the asset. There is no default: an example that
runs on the wrong asset reads a zero balance or sends the wrong token. The
values are `cbtc` and `beth`.

**Bridge examples** live per asset, in `examples/<asset>/`. They cover the
mint and redeem flows of one asset and ignore `ASSET`. Cargo registers each
one as `<asset>_<name>`. The CBTC bridge examples are in `examples/cbtc/`, for
example `cargo run --example cbtc_mint_flow`. The BETH bridge examples are in
`examples/beth/`.

| Token Standard example | What it does |
|---|---|
| `check_balance` | Balance and UTXO count |
| `token_client` | Read a position through `TokenClient` |
| `send`, `send_v2` | Send to another party |
| `list_incoming_offers`, `list_outgoing_offers` | List pending offers |
| `accept_transfers`, `reject_transfer`, `cancel_offers` | Settle pending offers |
| `allocate`, `withdraw_allocation` | Lock holdings into a settlement leg and take them back |
| `stream` | Send repeated transfers to one receiver |
| `consolidate_utxos`, `consolidate_utxos_v2`, `split_holding_v2` | Merge or split holdings |
| `batch_distribute`, `batch_distribute_v2`, `batch_with_callback`, `batch_with_callback_v2` | Distribute from a CSV file |
| `check_dars` | Check the participant's DAR packages |

| CBTC bridge example | What it does |
|---|---|
| `cbtc_credentials` | Find or accept the Minter credential |
| `cbtc_mint_flow` | Create a deposit account and get its BTC address |
| `cbtc_list_deposit_addresses` | List deposit accounts with their BTC addresses |
| `cbtc_redeem_flow` | Create a withdraw account and burn CBTC |
| `cbtc_list_withdraw_accounts` | List withdraw accounts |
| `cbtc_test_burn` | Burn a small amount through an existing withdraw account |
| `cbtc_check_withdraw_requests` | Poll for the payout records |

### Prerequisites for Mint & Redeem Examples

The mint and redeem examples require a Minter credential issued by the CBTC registrar. Run the examples in this order:

1. **`cbtc_credentials`** — Check for existing credentials, accept a pending Minter credential offer if needed
2. **`cbtc_mint_flow`** — Create a deposit account and get a BTC address (requires Minter credential)
3. **`cbtc_redeem_flow`** — Create a withdraw account and burn CBTC (requires Minter credential + CBTC balance)

These also require `ENVIRONMENT` in your `.env`, which supplies the BitSafe
API URL for the network.

### Credentials

List, accept, and manage CBTC Minter credentials:

```bash
cargo run --example cbtc_credentials
```

This example checks for existing Minter credentials. If none are found, it looks for pending credential offers from the registrar, accepts the first Minter offer, and displays the credential CID for use in other operations.

### Check DARs

Verify that the participant node holds every DAR package the library needs:

```bash
cargo run --example check_dars
```

The example scans the DAR files under `cbtc-dars/` and compares them against
the packages uploaded to the participant. It exits non-zero when one is
missing. It reads no registry and no BitSafe API, so it needs neither
`ENVIRONMENT` nor `PARTY_ID`.
This example needs the DAR files, which this repo adds in a later release under dars/; until then it finds no DARs.

### Mint CBTC Flow

Complete flow for minting CBTC from BTC:

```bash
cargo run --example cbtc_mint_flow
```

Creates a deposit account with Minter credentials, retrieves the BTC address, and displays account status. Requires a Minter credential (run `cbtc_credentials` first).

### List Deposit Addresses

List every deposit account for your party, with its Bitcoin address:

```bash
cargo run --example cbtc_list_deposit_addresses
```

The example reads the accounts from the ledger, then asks the BitSafe API for
each account's Bitcoin address. Use it to recover an address that
`cbtc_mint_flow` printed earlier.

### Redeem CBTC Flow

Complete flow for redeeming CBTC back to BTC:

```bash
cargo run --example cbtc_redeem_flow
```

Creates a withdraw account and submits a withdrawal. `submit_withdraw` checks the caller, the credentials, the transaction limits, the account's pending balance and the holdings before it sends anything. Requires a Minter credential and CBTC balance.

### List Withdraw Accounts

List every withdraw account for your party:

```bash
cargo run --example cbtc_list_withdraw_accounts
```

A withdraw account holds the destination Bitcoin address that the attestor
network pays out to. The example prints that address and the account's
pending balance. The pending balance is CBTC you burned that the attestor
network has not yet paid out.

### Test Burn CBTC

Burn a small amount of CBTC using an existing withdraw account:

```bash
cargo run --example cbtc_test_burn
```

### Check Withdraw Requests

Watch for the withdraw requests the attestor network creates:

```bash
cargo run --example cbtc_check_withdraw_requests
```

Submit a withdrawal first with `cbtc_redeem_flow`. The attestor network then
processes the pending balance and creates a `WithdrawRequest`. Each request
carries the Bitcoin transaction id of the payout, which `btc_tx_id()` returns.

The example polls every five seconds and does not stop on its own. Press
`Ctrl+C` to end it.

### BETH Bridge Examples

The BETH examples need a Minter credential issued by the BETH registrar.
Run them in this order:

1. **`beth_credentials`**: checks for a BETH Minter credential and accepts
   a pending offer if there is one.
2. **`beth_mint_flow`**: finds or creates a deposit account and prints the
   `depositETH` transaction for `DEPOSIT_AMOUNT_WEI`.
3. **`beth_redeem_flow`**: finds or creates a withdraw account for
   `DESTINATION_ETH_ADDRESS`, burns `WITHDRAW_AMOUNT` BETH and lists the
   withdraw requests.

```bash
cargo run --example beth_credentials
cargo run --example beth_mint_flow
cargo run --example beth_redeem_flow
```

`beth_mint_flow` sends nothing. You sign and send the printed transaction
with your own Ethereum wallet on the printed chain. The attestors mint BETH
after the deposit's block is finalized. An amount that is not a whole
multiple of 100000000 wei fails before the example logs in. A zero amount,
or an amount outside the deposit account's own limits, fails when the example
builds the call, after it finds or creates the account. The library does not
read the bridge's live `depositLimits()` or `paused()`; check both with your
Ethereum provider before you send.

### Check Balance

Check your balance and UTXO count for `ASSET`:

```bash
cargo run --example check_balance
```

### Read a Position with TokenClient

Read a party's balance, UTXO count and incoming offers through one client:

```bash
cargo run --example token_client
```

`TokenClient` stores the configuration that otherwise repeats on every call,
including the Token Standard version. This example writes nothing to the
ledger.

It reads one account rather than the whole party. A party whose holdings sit
under a labelled account sees zero here, while `check_balance` reports the
party's full total.

### Send

Send the asset to another party:

```bash
# Set the amount and receiver in .env or environment
export TRANSFER_AMOUNT=0.1
export RECEIVER_PARTY_ID="receiver-party::1220..."
cargo run --example send
```

### Send over Token Standard V2

The same transfer on the V2 entry point. V2 addresses accounts rather than bare
parties, so `sender` and `receiver` are `Account`s built with `Account::basic`.
That is the only difference from `send`:

```bash
export TRANSFER_AMOUNT=0.1
export RECEIVER_PARTY_ID="receiver-party::1220..."
cargo run --example send_v2
```

### List Incoming Offers

List all pending transfer offers where you are the receiver:

```bash
cargo run --example list_incoming_offers
```

This example lists all pending transfers waiting for you to accept.

### List Outgoing Offers

List all pending transfer offers where you are the sender:

```bash
cargo run --example list_outgoing_offers
```

This example shows all transfers you've sent that haven't been accepted yet.

### Accept Pending Transfers

Accept all pending transfers for your party:

```bash
cargo run --example accept_transfers
```

This example automatically fetches all pending TransferInstruction contracts and accepts them in a loop. Useful for automated acceptance of incoming transfers.

### Reject Incoming Transfers

Reject every incoming offer for the asset's instrument:

```bash
cargo run --example reject_transfer
```

Rejecting is the receiver's action, so this example authenticates as the
receiver. It reads `RECEIVER_PARTY_ID` and the `RECEIVER_KEYCLOAK_*`
variables. Where the receiver shares the sender's host and realm, it falls
back to `LEDGER_HOST`, `KEYCLOAK_HOST` and `KEYCLOAK_REALM`.

Create an offer first. Run `send` from the sender to another party. A
transfer to your own party settles on submission and leaves nothing to
reject. `send` sends to `RECEIVER_PARTY_ID`, so the sender must set that
variable to the party that rejects.

The example lists the pending offers and rejects each one. The rejection
returns the tokens to the sender. With no offers it says so and exits 0.

Token Standard V2 needs no separate example. `reject::v2` re-exports V1's
`Params` unchanged, so only the module path differs.

### Cancel Pending Transfers

Cancel all pending outgoing transfers that haven't been accepted:

```bash
cargo run --example cancel_offers
```

This example withdraws all transfer offers you've sent that are still pending, returning the tokens to your account.

### Allocate for DvP

Lock holdings into one leg of a Delivery-versus-Payment settlement:

```bash
export RECEIVER_PARTY_ID="receiver-party::1220..."
export EXECUTOR_PARTY_ID="venue-party::1220..."
export ALLOCATE_AMOUNT=0.1
cargo run --example allocate
```

The settlement executor settles every leg atomically later. The example sets
`allocateBefore` 24 hours ahead and `settleBefore` 48 hours ahead, and names
the settlement `cbtc-dvp-example` unless `SETTLEMENT_REF_ID` says otherwise.
It selects the sender's holdings itself.

The registry answers one of two ways. It creates the allocation, and the
example prints the contract id and the command that reclaims it. Or it
creates an allocation instruction, which needs a further step and cannot be
withdrawn. The example prints the instruction id in that case, because that
id is the only handle on it.

### Withdraw a DvP Allocation

Take back the holdings that an allocation locked:

```bash
ALLOCATION_CONTRACT_ID=<cid> cargo run --example withdraw_allocation
```

`allocate` locks holdings into a settlement leg. The sender takes those
holdings back by withdrawing the allocation, until the executor settles the
leg. `cancel` works the same way for the executor.

You supply the allocation's contract id, because nothing in the crate lists
allocations. `allocate` prints that id when it succeeds, together with
the command that reclaims it. Copy that line.

### Stream

Stream the asset to a single receiver multiple times:

```bash
# Set the streaming parameters
export RECEIVER_PARTY_ID="receiver-party::1220..."
export TRANSFER_COUNT=10
export TRANSFER_AMOUNT=0.001
cargo run --example stream
```

This example sends multiple transfers to the same receiver, useful for streaming payments or testing repeated transfers.

### Consolidate UTXOs

Check and consolidate UTXOs if needed:

```bash
# Optional: set custom threshold (default is 10)
export CONSOLIDATION_THRESHOLD=8
cargo run --example consolidate_utxos
```

### Consolidate UTXOs over Token Standard V2

```bash
cargo run --example consolidate_utxos_v2
```

The V2 counterpart of `consolidate_utxos`. One thing differs: V1 takes the
party as a string, and V2 takes a `bitsafe_token::Account`. The example uses
`Account::basic`, the unlabelled account every party owns. It does not reach
a labelled account.

### Split a Holding over Token Standard V2

Turn one holding into several of the amounts you name, plus change:

```bash
# Optional: name the outputs. The default is a single output of 0.001.
export SPLIT_AMOUNTS=0.001,0.002
cargo run --example split_holding_v2
```

The example reads the party's holdings, picks the largest, and splits it. It
reads and splits the same account, because the registry rejects a holding
that a labelled account owns when the split names the basic one.

Splitting has no V1 example. The library offers `bitsafe_token::split::submit` for V1.

### Batch Distribute

Distribute the asset to multiple recipients from a CSV file:

```bash
# Create recipients.csv with format:
# receiver,amount
# party1::1220...,5.0
# party2::1220...,3.5

# Run the example
cargo run --example batch_distribute

# Or specify a custom CSV path
export RECIPIENTS_CSV=my_recipients.csv
cargo run --example batch_distribute
```

### Batch Distribute over Token Standard V2

```bash
cargo run --example batch_distribute_v2
```

The V2 counterpart of `batch_distribute`, reading the same CSV file. The
format does not change between versions: each row names a bare party, and the
library lifts it to a basic account. The sender differs, because V2 takes a
`bitsafe_token::Account` where V1 takes a party string.

### Batch Distribute with Callback

Distribute to multiple recipients with real-time result logging:

```bash
cargo run --example batch_with_callback
```

This example demonstrates the callback feature, which allows you to process transfer results as they complete. The callback writes one line per transfer to a timestamped log file.

### Batch Distribute with Callback over Token Standard V2

```bash
cargo run --example batch_with_callback_v2
```

The V2 counterpart of `batch_with_callback`. The callback fires once per
transfer under either version, and `distribute::v2` returns the same result
as V1. V2 takes a `bitsafe_token::Account` for the sender and for each recipient.

## Transfer Result Callbacks

The batch distribution system supports optional callbacks that are invoked after each transfer completes (whether successful or failed). This allows for real-time processing of transfer results.

### TransferResult Structure

Each callback receives a `TransferResult` with the following fields:

```rust
pub struct TransferResult {
    pub success: bool,              // Whether the transfer succeeded
    pub transfer_index: usize,      // Index in the batch (0-based)
    pub receiver: String,           // Recipient address
    pub amount: String,             // Transfer amount
    pub transfer_offer_cid: Option<String>,  // Contract ID if successful
    pub update_id: Option<String>,  // Canton ledger update ID if successful
    pub reference: Option<String>,  // Unique reference ID (base64 encoded)
    pub raw_response: Option<String>, // Full JSON response from ledger
    pub error: Option<String>,      // Error message if failed
}
```

### Use Cases

1. **Real-time Logging**: Write results to a file or logging service as they complete
2. **Database Updates**: Store transfer records in a database immediately
3. **Progress Tracking**: Update UI or monitoring dashboards
4. **Notifications**: Send alerts for failed transfers
5. **Custom Retry Logic**: Implement application-specific retry strategies

### Basic Usage

```rust
use std::pin::Pin;
use std::future::Future;

let callback = Box::new(|result: bitsafe_token::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        if result.success {
            println!("✅ Transfer succeeded: {} to {}", result.amount, result.receiver);
            // Log to database, send notification, etc.
        } else {
            println!("❌ Transfer failed: {}", result.error.unwrap_or_default());
            // Handle failure (retry, alert, etc.)
        }
    })
}) as Box<bitsafe_token::transfer::TransferResultCallback>;

let result = bitsafe_token::distribute::submit(bitsafe_token::distribute::Params {
    // ... other params
    on_transfer_complete: Some(callback),
})
.await?;
```

### File Logging Example (One Line Per Event)

```rust
use std::fs::OpenOptions;
use std::io::Write;

let log_file = format!("transfer_results_{}.log", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

let callback = Box::new(move |result: bitsafe_token::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let log_file = log_file.clone();
    Box::pin(async move {
        let status = if result.success { "SUCCESS" } else { "FAILED" };
        let reference = result.reference.as_deref().unwrap_or("N/A");
        let offer_cid = result.transfer_offer_cid.as_deref().unwrap_or("N/A");
        let update_id = result.update_id.as_deref().unwrap_or("N/A");
        let error = result.error.as_deref().unwrap_or("N/A");

        let log_line = format!(
            "{} | {} | idx={} | to={} | amount={} | ref={} | offer={} | update_id={} | error={}\n",
            chrono::Utc::now().to_rfc3339(),
            status,
            result.transfer_index,
            result.receiver,
            result.amount,
            reference,
            offer_cid,
            update_id,
            error
        );

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    })
}) as Box<bitsafe_token::transfer::TransferResultCallback>;
```

Example output:

```
2025-01-10T15:23:45.123Z | SUCCESS | idx=0 | to=merchant::1220... | amount=0.001 | ref=YmF0Y2gtMTIz... | offer=00123... | update_id=12208abc... | error=N/A
2025-01-10T15:23:46.456Z | FAILED | idx=1 | to=validator::5678... | amount=0.002 | ref=YmF0Y2gtNDU2... | offer=N/A | update_id=N/A | error=Insufficient funds
```

### Database Logging Example

```rust
let db_pool = /* your DB connection pool */;

let callback = Box::new(move |result: bitsafe_token::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let pool = db_pool.clone();
    Box::pin(async move {
        sqlx::query!(
            "INSERT INTO transfer_logs (receiver, amount, success, reference, raw_response, error)
             VALUES ($1, $2, $3, $4, $5, $6)",
            result.receiver,
            result.amount,
            result.success,
            result.reference,
            result.raw_response,
            result.error
        )
        .execute(&pool)
        .await
        .unwrap();
    })
}) as Box<bitsafe_token::transfer::TransferResultCallback>;
```

### Progress Tracking Example

```rust
use std::sync::{Arc, Mutex};

let progress = Arc::new(Mutex::new(0));
let total = recipients.len();

let callback = Box::new(move |result: bitsafe_token::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let progress = progress.clone();
    Box::pin(async move {
        let mut count = progress.lock().unwrap();
        *count += 1;
        println!("Progress: {}/{}", count, total);

        if result.success {
            println!("  ✓ {} transferred", result.amount);
        } else {
            println!("  ✗ Failed: {}", result.error.unwrap_or_default());
        }
    })
}) as Box<bitsafe_token::transfer::TransferResultCallback>;
```

### No Callback (Default)

If you don't need callbacks, simply pass `None`:

```rust
let result = bitsafe_token::distribute::submit(bitsafe_token::distribute::Params {
    // ... other params
    on_transfer_complete: None,  // No callback
})
.await?;
```

### Important Notes

1. **Callbacks are async**: They can perform async operations like database writes
2. **Callbacks are called sequentially**: One completes before the next transfer starts
3. **Errors in callbacks don't stop the batch**: If a callback panics, it's isolated
4. **The reference field** contains the base64-encoded unique ID: `base64(reference_base + sender + receiver)`
5. **The update_id field** contains the Canton ledger's unique update ID for successful transfers (used for tracking and idempotency)
6. **The raw_response field** contains the full JSON response from the Canton ledger (for successful submissions)

See `batch_with_callback.rs` for a complete working example.

## Environment Variables

Every variable below also appears in `.env.example`.

Required for all examples:

- `KEYCLOAK_HOST` - Your Keycloak host, without `/auth`
- `KEYCLOAK_REALM` - Your Keycloak realm
- `KEYCLOAK_CLIENT_ID` - Client ID
- `KEYCLOAK_USERNAME` - Username
- `KEYCLOAK_PASSWORD` - Password
- `LEDGER_HOST` - Canton participant JSON ledger API URL, including the API path

Required for every example except `check_dars`:

- `PARTY_ID` - Your party ID

Required for every example that touches a network:

- `ENVIRONMENT` - `devnet`, `testnet` or `mainnet`, which supplies the
  registrar party, the registry URL and the BitSafe API URL.

Required for every Token Standard example except `check_dars`:

- `ASSET` - The asset to work on: `cbtc` or `beth`.

Three examples read neither `ENVIRONMENT` nor `ASSET` and need no network
configuration: `check_dars`, `cbtc_list_withdraw_accounts` and
`cbtc_check_withdraw_requests`.

Optional overrides, for a custom network. An empty value counts as unset:

- `REGISTRAR_PARTY` - Overrides the asset's registrar party
- `REGISTRY_URL` - Overrides the Canton registry URL

Mint and redeem:

- `DESTINATION_BTC_ADDRESS` - Payout address for `cbtc_redeem_flow`
- `DESTINATION_ETH_ADDRESS` - Payout address for `beth_redeem_flow`. It has no default.
- `DEPOSIT_AMOUNT_WEI` - The deposit `beth_mint_flow` builds, in wei (default: `10000000000000000`, 0.01 ETH)
- `WITHDRAW_AMOUNT` - Amount to burn, in the asset's unit (default: `0.001` for `cbtc_redeem_flow`, `0.01` for `beth_redeem_flow`)

Transfers:

- `TRANSFER_AMOUNT` - Amount to send (default: 0.1)
- `TRANSFER_COUNT` - Number of transfers in the `stream` example
- `RECEIVER_PARTY_ID` - Receiver party for `send`, `send_v2`, `stream` and `allocate`, and the rejecting party for `reject_transfer`
- `RECIPIENTS_CSV` - Path to CSV file for batch distribution (default: recipients.csv)
- `SPLIT_AMOUNTS` - Comma-separated outputs for the split example (default: 0.001)
- `CONSOLIDATION_THRESHOLD` - UTXO threshold for consolidation (default: 10)

Allocations:

- `ALLOCATE_AMOUNT` - Amount to allocate for DvP (default: 0.1)
- `EXECUTOR_PARTY_ID` - The settlement executor, also called the venue
- `SETTLEMENT_REF_ID` - Settlement reference id (default: cbtc-dvp-example)
- `ALLOCATION_CONTRACT_ID` - The allocation to withdraw. Set it per run, on the command line.

The receiver's own login, for `reject_transfer`:

- `RECEIVER_KEYCLOAK_CLIENT_ID` - Client ID for the receiver
- `RECEIVER_KEYCLOAK_USERNAME` - Username for the receiver
- `RECEIVER_KEYCLOAK_PASSWORD` - Password for the receiver
- `RECEIVER_LEDGER_HOST` - Optional. Defaults to `LEDGER_HOST`
- `RECEIVER_KEYCLOAK_HOST` - Optional. Defaults to `KEYCLOAK_HOST`
- `RECEIVER_KEYCLOAK_REALM` - Optional. Defaults to `KEYCLOAK_REALM`

Logging:

- `RUST_LOG` - Log filter for `env_logger`
