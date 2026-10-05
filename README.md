# bitsafe-token

Rust client library for BitSafe bridged assets on Canton Network. It serves
CBTC and BETH with one API shape, and re-exports the Canton Token Standard
operations of [canton-lib](https://github.com/DLC-link/canton-lib) under the
same names [cbtc-lib](https://github.com/DLC-link/cbtc-lib) uses.

Status: under construction. CBTC is under `tokens::cbtc`, and BETH is under `tokens::beth`.

## Networks

`Network` is a plain enum: `Devnet`, `Testnet`, `Mainnet`. It carries the
values that do not depend on the asset. Each asset module maps a `Network`
to its own registrar party.

| Network | Utility registry | BitSafe API |
| --- | --- | --- |
| devnet | https://api.utilities.digitalasset-dev.com | https://api.devnet.bitsafe.finance |
| testnet | https://api.utilities.digitalasset-staging.com | https://api.testnet.bitsafe.finance |
| mainnet | https://api.utilities.digitalasset.com | https://api.mainnet.bitsafe.finance |

## Layout

- `tokens::<asset>`: the whole asset-facing API. One module per asset.
- `kits`, `flows`: crate-private building blocks. They never appear in a
  public path.
- The crate root: the `canton-lib` re-exports, `Network`, `AssetInfo`,
  `check_dars`.
- `dars/`: the DAR packages a participant needs for each asset, and the
  script that uploads them. See [`dars/README.md`](dars/README.md).

## Development

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The dependency rule: `tokens` may use `flows` and `kits`, `flows` may use
`kits`, and nothing imports upward. Review checks it.

## Localnet suite

The localnet suite runs the CBTC and then the BETH credentials, mint and
redeem flows against a local Canton sandbox. It needs Docker. Two commands
start the sandbox and run the suite:

```sh
docker compose -f localnet/docker-compose.yml up -d --wait
cargo test --lib localnet -- --ignored --nocapture --test-threads=1
```

The first pull of the sandbox image takes several minutes. The sandbox then
needs a few more minutes to bootstrap. Each run allocates fresh parties, so a
rerun against a running sandbox works. The suite uses the JSON ledger API at
`http://localhost:7575`; set `LOCALNET_LEDGER_HOST` to use another one.

One command stops the sandbox and deletes its data:

```sh
docker compose -f localnet/docker-compose.yml down -v
```

## License

Apache-2.0
