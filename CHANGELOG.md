# Changelog

All notable changes to `bitsafe-token` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-10-21

### Fixed

- `credentials::accept_credential_offer` exercises the UserService by the
  package-name template id, through `canton-lib` 0.12.0. The participant then
  picks a package version that every host vets. With 0.2.0, the accept used
  the package version that the UserService was created with. A participant
  that did not vet that version rejected the accept with
  `INVALID_PRESCRIBED_SYNCHRONIZER_ID`.

### Changed — breaking

- `AcceptCredentialOfferParams` loses its `user_service_template_id` field.
  Drop it at the call site; the `credentials` examples show the new shape.
- The crate uses `canton-lib` 0.12.0 instead of 0.11.0. A project that also
  depends on a `canton-lib` crate must pin `v0.12.0`. Two different pins make
  Cargo build two copies of the shared types, and then the types do not
  match.

### Security

- The lockfile moves `rustls` to 0.23.46. This closes RUSTSEC-2026-0346 to
  RUSTSEC-2026-0349.
- `canton-lib` 0.12.0 moves `zip` to 9.0.5. This closes RUSTSEC-2026-0367 to
  RUSTSEC-2026-0369.
- The lockfile moves `rustls-webpki` to 0.103.16, which closes
  RUSTSEC-2026-0382. It moves `webpki-root-certs` to 1.0.10, which closes
  RUSTSEC-2026-0383.

## [0.2.0] - 2026-10-09

### Changed

- The crate uses `canton-lib` 0.11.0 instead of 0.10.0. `canton-lib` now
  uses `canton-api-client` 3.6.1 instead of 3.6.0.
- A project that also depends on a `canton-lib` crate must pin `v0.11.0`.
  With a `v0.10.0` pin, Cargo cannot resolve `canton-api-client` and the
  build fails. A direct `canton-api-client` dependency must be
  `3.6.1-0.1.0` for the same reason.
- The re-exported `active_contracts::get` returns the Ledger API error when
  the participant rejects the request. With 0.10.0 it returned an empty
  list, so a rejected balance query reported 0.

## [0.1.0] - 2026-10-06

The first release. `bitsafe-token` replaces `cbtc-lib` and adds BETH.

### Added

- `tokens::cbtc` and `tokens::beth`: the mint and burn flows of both assets,
  with one API shape. Each module maps a `Network` to its own registrar.
- `tokens::beth::mint::deposit_call`: builds the `depositETH` transaction
  for a deposit account. It checks the network, the account id, the amount,
  the wei unit and the account limits. It sends nothing.
- `check_dars`: compares one asset's DAR set under `dars/` with a
  participant.
- `dars/`: the DAR packages for CBTC and BETH, and `dars/upload_dars.sh
  <asset>`, which uploads them and stops at the first failed upload.
- Examples for both assets, a localnet integration suite and a contract test
  of the live BitSafe API.

### Changed from `cbtc-lib`

- The crate is `bitsafe-token` (`bitsafe_token` in code), not `cbtc`.
- The CBTC flows move from `mint_redeem` to `tokens::cbtc::{mint, redeem}`,
  and `CBTC_TICKER` is now `tokens::cbtc::TICKER`.
- The DAR folder `cbtc-dars/` is now `dars/`. `upload_dars.sh` takes the
  asset as its argument, and reads `jwt_token` and `canton_admin_api_url`
  from the environment.
- The crate uses `canton-lib` 0.10.0 instead of 0.9.0. Its active-contract
  queries and update streams send `eventFormat` and `updateFormat`, which
  Canton 3.6.1 requires. The `ledger` re-export no longer has
  `submit::wait_for_transaction_tree` or `common::TransactionFilter`.
- The dependency DAR set changed: `splice-amulet` 0.1.16 instead of 0.1.17,
  `splice-util` 0.1.4 instead of 0.1.5, and newer Canton Network Utility
  versions.
- The `check_dars` example reads `ASSET`, as every Token Standard example
  does.
- The `cbtc_redeem_flow` example requires `DESTINATION_BTC_ADDRESS`, and
  reuses only a withdraw account with that destination.

### Removed from `cbtc-lib`

- The template-id constants of `mint_redeem::constants`, such as
  `DEPOSIT_ACCOUNT_TEMPLATE_ID`.
- `mint_redeem::attestor::get_token_standard_contracts`.
- The deprecated `issuer_credential` of the BitSafe API. The burn sends an
  empty `issuer-credentials` list.
- `cbtc-tui`, the terminal UI.
