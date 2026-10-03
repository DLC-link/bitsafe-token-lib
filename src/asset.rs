//! The one shared contract between assets: a plain data constant that tooling
//! such as `check_dars`, the examples and a later TUI iterate over.

use crate::Network;

/// An asset's identity: the facts tooling needs without any flow.
#[derive(Debug, Clone, Copy)]
pub struct AssetInfo {
    /// The instrument id, `"CBTC"` or `"BETH"`.
    pub ticker: &'static str,
    /// The registrar party that administers the instrument on a network.
    pub registrar: fn(Network) -> &'static str,
    /// The credential claim the account and burn choices require.
    pub minter_claim: (&'static str, &'static str),
    /// The DAR directories `check_dars` compares, dependencies first.
    pub dar_dirs: &'static [&'static str],
}

impl AssetInfo {
    /// Maps a registrar party back to its network, for callers and a later
    /// TUI. `None` when no network of this asset uses that party.
    pub fn network_of(&self, registrar: &str) -> Option<Network> {
        Network::ALL
            .into_iter()
            .find(|network| (self.registrar)(*network) == registrar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn fake_registrar(network: Network) -> &'static str {
        match network {
            Network::Devnet => "fake::devnet",
            Network::Testnet => "fake::testnet",
            Network::Mainnet => "fake::mainnet",
        }
    }

    const FAKE: AssetInfo = AssetInfo {
        ticker: "FAKE",
        registrar: fake_registrar,
        minter_claim: ("hasFAKERole", "Minter"),
        dar_dirs: &["dars/dependencies", "dars/fake"],
    };

    #[test]
    fn network_of_maps_each_registrar_back() {
        assert_eq!(FAKE.network_of("fake::devnet"), Some(Network::Devnet));
        assert_eq!(FAKE.network_of("fake::testnet"), Some(Network::Testnet));
        assert_eq!(FAKE.network_of("fake::mainnet"), Some(Network::Mainnet));
    }

    #[test]
    fn network_of_returns_none_for_an_unknown_registrar() {
        assert_eq!(FAKE.network_of(""), None);
        assert_eq!(FAKE.network_of("fake::Devnet"), None);
        assert_eq!(FAKE.network_of("other::devnet"), None);
    }

    #[test]
    fn all_lists_every_supported_asset_once() {
        let tickers: Vec<&str> = crate::tokens::ALL.iter().map(|info| info.ticker).collect();
        assert_eq!(tickers, vec!["CBTC"]);
    }
}
