//! The networks every BitSafe asset is deployed on, and the values that do
//! not depend on the asset.
//!
//! The registrar party differs per asset, so it lives in each asset module
//! under `tokens`, not here. The registry URLs are re-exports from
//! `canton-lib`. The BitSafe API URLs have no upstream home and must not get
//! one: `canton-lib` is a generic Canton library.

use std::fmt;
use std::str::FromStr;

/// A BitSafe deployment. Carries only the values that do not depend on
/// the asset. Each asset module maps a `Network` to its own registrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    Devnet,
    Testnet,
    Mainnet,
}

impl Network {
    /// Every network, in declaration order: devnet, testnet, mainnet.
    pub const ALL: [Network; 3] = [Self::Devnet, Self::Testnet, Self::Mainnet];

    /// Digital Asset's utility registry for this network.
    pub const fn registry_url(self) -> &'static str {
        match self {
            Self::Devnet => registry::consts::DEVNET_REGISTRY_URL,
            Self::Testnet => registry::consts::TESTNET_REGISTRY_URL,
            Self::Mainnet => registry::consts::MAINNET_REGISTRY_URL,
        }
    }

    /// BitSafe's API for this network. Mint and redeem need it.
    pub const fn bitsafe_api_url(self) -> &'static str {
        match self {
            Self::Devnet => "https://api.devnet.bitsafe.finance",
            Self::Testnet => "https://api.testnet.bitsafe.finance",
            Self::Mainnet => "https://api.mainnet.bitsafe.finance",
        }
    }

    /// The name [`Display`] writes and [`FromStr`] accepts.
    const fn name(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// [`Network::from_str`] was given a name that is not one of the three.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNetworkError {
    /// The name that was rejected.
    pub name: String,
}

impl fmt::Display for ParseNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown network {:?}: expected devnet, testnet or mainnet",
            self.name
        )
    }
}

impl std::error::Error for ParseNetworkError {}

impl FromStr for Network {
    type Err = ParseNetworkError;

    /// Exact, lowercase match.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|network| network.name() == s)
            .ok_or_else(|| ParseNetworkError {
                name: s.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Six assertions, each against a literal spelled out here, because
    /// comparing a method against the constant it returns cannot fail.
    #[test]
    fn each_network_returns_its_own_two_values() {
        assert_eq!(
            Network::Devnet.registry_url(),
            "https://api.utilities.digitalasset-dev.com"
        );
        assert_eq!(
            Network::Testnet.registry_url(),
            "https://api.utilities.digitalasset-staging.com"
        );
        assert_eq!(
            Network::Mainnet.registry_url(),
            "https://api.utilities.digitalasset.com"
        );
        assert_eq!(
            Network::Devnet.bitsafe_api_url(),
            "https://api.devnet.bitsafe.finance"
        );
        assert_eq!(
            Network::Testnet.bitsafe_api_url(),
            "https://api.testnet.bitsafe.finance"
        );
        assert_eq!(
            Network::Mainnet.bitsafe_api_url(),
            "https://api.mainnet.bitsafe.finance"
        );
    }

    /// `Display` and `FromStr` are inverses over all three variants.
    #[test]
    fn display_and_from_str_round_trip() {
        for network in Network::ALL {
            let name = network.to_string();
            assert_eq!(
                name.parse::<Network>()
                    .expect("Display's output must parse"),
                network
            );
        }
        assert_eq!("devnet".parse::<Network>().unwrap(), Network::Devnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!(Network::Devnet.to_string(), "devnet");
        assert_eq!(Network::Testnet.to_string(), "testnet");
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
    }

    /// `"Devnet"` pins the case-sensitivity decision: these names are config
    /// keys in `cbtc-tui`, so relaxing it would break saved files.
    #[test]
    fn from_str_rejects_a_name_that_is_not_one_of_the_three() {
        for name in ["", "Devnet", "DEVNET", "local", "dev", "devnet "] {
            let error = name
                .parse::<Network>()
                .expect_err("only the three lowercase names are valid");
            let message = error.to_string();
            assert!(message.contains("devnet"), "{message}");
            assert!(message.contains("testnet"), "{message}");
            assert!(message.contains("mainnet"), "{message}");
        }
    }

    /// `ALL` holds three distinct variants. The compiler catches a variant
    /// added to `Network`, but not one added with `ALL` left alone.
    #[test]
    fn all_holds_every_variant_once() {
        assert_eq!(Network::ALL.len(), 3);
        let mut seen = std::collections::HashSet::new();
        for network in Network::ALL {
            assert!(seen.insert(network), "{network} appears twice in ALL");
        }
    }

    /// Every value this module resolves also appears in the README. One
    /// direction only: it does not prove the value sits in the right block.
    #[test]
    fn the_readme_documents_every_per_network_value() {
        let readme = include_str!("../README.md");
        for network in Network::ALL {
            for value in [network.registry_url(), network.bitsafe_api_url()] {
                assert!(
                    readme.contains(value),
                    "README.md does not document {value}"
                );
            }
        }
    }
}
