//! Every name `cbtc-lib` re-exported from `canton-lib` is reachable here under
//! the same path. A missing name is a compile error, which is the test.

#[allow(unused_imports)]
use bitsafe_token::{
    DamlDecimal, DistributeParams, InstrumentId, KeycloakConfig, Meta, SendParams, SplitParams,
    TokenClient, TokenClientConfig, TokenStandardVersion, Transfer, accept, active_contracts,
    allocation, batch, cancel_offers, consolidate, credentials, dar_check, distribute, holding,
    reject, split, transfer, types, utils,
};

#[test]
fn the_re_exports_compile() {
    let _: fn() -> bitsafe_token::types::v2::Transfer = || unreachable!();
    let _: Option<bitsafe_token::Account> = None;
}
