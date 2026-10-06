# Changelog

All notable changes to `bitsafe-token` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
