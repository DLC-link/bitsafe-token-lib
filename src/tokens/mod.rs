//! One module per asset. Each module is the whole public API of that asset.

use crate::AssetInfo;

pub mod cbtc;

/// Every asset this crate supports, for tooling that iterates over assets.
pub const ALL: &[&AssetInfo] = &[&cbtc::INFO];
