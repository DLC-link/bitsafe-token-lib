#![allow(dead_code)]
//! The values the examples resolve from the environment, in one place.
//!
//! `mod shared;` makes this a private module of each example crate, so a
//! function that example does not call is dead code there, hence the
//! attribute above.

use std::env;

use bitsafe_token::{AssetInfo, InstrumentId, Network, tokens};

/// The asset a Token Standard example works on, from `ASSET`. There is no
/// default: a wrong one reads a zero balance or sends the wrong token.
pub fn asset() -> &'static AssetInfo {
    let name = non_blank("ASSET").unwrap_or_else(|| panic!("set ASSET to {}", asset_names()));
    tokens::ALL
        .iter()
        .copied()
        .find(|info| info.ticker.eq_ignore_ascii_case(&name))
        .unwrap_or_else(|| panic!("ASSET {name:?} is not one of {}", asset_names()))
}

fn asset_names() -> String {
    tokens::ALL
        .iter()
        .map(|info| info.ticker.to_lowercase())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The network from `ENVIRONMENT`. The mint and redeem calls take it.
pub fn network() -> Network {
    let name = non_blank("ENVIRONMENT")
        .unwrap_or_else(|| panic!("set ENVIRONMENT to devnet, testnet or mainnet"));
    name.parse()
        .unwrap_or_else(|e| panic!("ENVIRONMENT is not a network: {e}"))
}

/// The asset's registrar, from `REGISTRAR_PARTY` or `ENVIRONMENT`.
pub fn registrar(asset: &AssetInfo) -> String {
    if let Some(value) = non_blank("REGISTRAR_PARTY") {
        if let Some(warning) = cross_network_warning("REGISTRAR_PARTY", &value, asset) {
            eprintln!("{warning}");
        }
        return value;
    }
    (asset.registrar)(network()).to_string()
}

/// The asset's instrument, with the resolved registrar as its admin.
pub fn instrument(asset: &AssetInfo) -> InstrumentId {
    InstrumentId {
        admin: registrar(asset),
        id: asset.ticker.to_string(),
    }
}

/// Digital Asset's utility registry, from `REGISTRY_URL` or `ENVIRONMENT`.
pub fn resolve_registry_url() -> String {
    if let Some(value) = non_blank("REGISTRY_URL") {
        return value;
    }
    network().registry_url().to_string()
}

/// A warning when `value` is the asset's registrar on another named network
/// while `ENVIRONMENT` names a different one. A custom deployment stays silent.
pub fn cross_network_warning(variable: &str, value: &str, asset: &AssetInfo) -> Option<String> {
    let chosen: Network = non_blank("ENVIRONMENT")?.parse().ok()?;
    let named = asset.network_of(value)?;
    if named == chosen {
        return None;
    }
    Some(format!(
        "warning: {variable} holds {named}'s value, but ENVIRONMENT is {chosen}. \
         The explicit variable wins, so this run mixes two networks."
    ))
}

/// `variable`'s value, trimmed; whitespace and the empty string count as unset.
fn non_blank(variable: &str) -> Option<String> {
    env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
