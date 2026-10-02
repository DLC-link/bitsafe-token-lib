//! Client library for every BitSafe bridged asset on Canton.
//!
//! The Token Standard operations come from `canton-lib` unchanged and are
//! re-exported here under the same names `cbtc-lib` used. Each asset's own
//! flows live under `tokens`.

mod network;
pub use network::{Network, ParseNetworkError};
mod asset;
pub use asset::AssetInfo;

pub(crate) mod flows;
pub(crate) mod kits;
/// Every supported asset. The whole asset-facing API lives here.
pub mod tokens;

/// The parameter types the Token Standard operations take.
///
/// `v2::Transfer` cannot sit at the crate root, because `transfer` at the
/// root is already `token::transfer`. So a caller reaches it as `types::v2`.
/// Each name here appears once: the root carries the short names below, and
/// this module carries the rest, so no type is reachable by two paths.
pub mod types {
    pub use common::allocation;
    pub use common::transfer::{DisclosedContract, v2};
    pub use common::transfer_factory;
}

pub use common::decimal::DamlDecimal;
pub use common::instrument::InstrumentId;
pub use common::transfer::{Meta, Transfer, v2::Account};
pub use token::{
    DistributeParams, KeycloakConfig, SendParams, SplitParams, TokenClient, TokenClientConfig,
    TokenStandardVersion, accept, active_contracts, allocation, batch, cancel_offers, consolidate,
    credentials, dar_check, distribute, holding, reject, split, transfer, utils,
};
