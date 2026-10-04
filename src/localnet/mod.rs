//! The localnet integration suite. It runs the CBTC flows against canton's
//! published sandbox image, with a fresh plain test registrar per run. Start
//! the sandbox with `docker compose -f localnet/docker-compose.yml up -d
//! --wait`, then run `cargo test --lib localnet -- --ignored --nocapture
//! --test-threads=1`.

mod cbtc;
mod fixture;
mod ledger;
