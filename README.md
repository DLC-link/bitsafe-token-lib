# bitsafe-token

Rust client library for BitSafe bridged assets on Canton Network. It serves
CBTC and BETH with one API shape, and re-exports the Canton Token Standard
operations of [canton-lib](https://github.com/DLC-link/canton-lib) under the
same names [cbtc-lib](https://github.com/DLC-link/cbtc-lib) uses.

Status: under construction. The crate builds and has no asset modules yet.

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
- The crate root: the `canton-lib` re-exports, `Network`, `AssetInfo`.

## Development

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The dependency rule: `tokens` may use `flows` and `kits`, `flows` may use
`kits`, and nothing imports upward. Review checks it.

## License

Apache-2.0
