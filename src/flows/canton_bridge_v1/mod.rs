//! The first flow family: the account, mint and burn flows that CBTC and
//! BETH share, written once over the kits.
//!
//! The asset is a type parameter. [`CantonBridgeV1`] names every Daml string
//! that differs between the asset packages. A generic flow never takes a
//! `Network`: it takes the registrar party and the BitSafe API URL that the
//! asset's public wrapper resolved. A family exists only when two assets
//! share it; an asset with other choices writes its own flows on the kits.

pub(crate) mod models;

/// The Daml names of one asset that has CBTC's choice shape.
///
/// Every value is a compile-time literal, the same strings `cbtc-lib` held in
/// its constants. The trait gets a new constant only when every asset in the
/// family has the thing it names. A field that only one asset has is an
/// inherent method on that asset's concrete model.
pub(crate) trait CantonBridgeV1 {
    const TICKER: &'static str;
    /// The asset's path segment in the BitSafe API: `cbtc` or `beth`.
    const API_PATH: &'static str;
    const DEPOSIT_ACCOUNT: &'static str;
    const DEPOSIT_ACCOUNT_RULES: &'static str;
    const WITHDRAW_ACCOUNT: &'static str;
    const WITHDRAW_ACCOUNT_RULES: &'static str;
    const WITHDRAW_REQUEST: &'static str;
    const CREATE_DEPOSIT_ACCOUNT_CHOICE: &'static str;
    const CREATE_WITHDRAW_ACCOUNT_CHOICE: &'static str;
    const WITHDRAW_CHOICE: &'static str;
    const LAST_PROCESSED_BLOCK_FIELD: &'static str;
    const DESTINATION_ADDRESS_FIELD: &'static str;
    /// The credential claim the account and burn choices require.
    const MINTER_CLAIM: (&'static str, &'static str);
    /// The reason the burn records in its metadata.
    const WITHDRAW_REASON: &'static str;

    /// Checks a destination address before submission, so a bad address
    /// fails locally with a clear message.
    fn validate_destination(address: &str) -> Result<(), String>;
}

/// Fails when a model's registrar is not the registrar of the network the
/// caller named. A devnet account in a mainnet call fails here.
pub(crate) fn check_registrar(actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "network mismatch: account registrar {}, expected {}",
            actual, expected
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_registrar_names_both_parties_on_a_mismatch() {
        assert!(check_registrar("cbtc-network::a", "cbtc-network::a").is_ok());
        assert_eq!(
            check_registrar("cbtc-network::a", "cbtc-network::b").unwrap_err(),
            "network mismatch: account registrar cbtc-network::a, expected cbtc-network::b"
        );
    }
}
