//! The EVM chain namespace: what holds for any EVM chain. It binds no
//! contract. Each contract has its own submodule, which may use the
//! chain-level code here.

use alloy_primitives::Address;

pub(crate) mod evm_asset_bridge;

/// Checks an EVM address: `0x` and 40 hex digits. An address with an
/// uppercase hex letter must match its EIP-55 checksum. An all-lowercase
/// address carries no checksum and passes. The caller's text is never
/// rewritten.
pub(crate) fn validate_address(address: &str) -> Result<(), String> {
    let digits = address.strip_prefix("0x").unwrap_or_default();
    if digits.len() != 40 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("address {address:?} is not 0x and 40 hex digits"));
    }
    if digits.bytes().any(|byte| byte.is_ascii_uppercase())
        && Address::parse_checksummed(address, None).is_err()
    {
        return Err(format!(
            "address {address} does not match its EIP-55 checksum"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test vectors of EIP-55 itself, all with a valid checksum.
    #[test]
    fn the_eip_55_vectors_pass() {
        for address in [
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "0x8617E340B3D01FA5F11F306F4090FD50E238070D",
            "0xde709f2102306220921060314715629080e2fb77",
            "0x27b1fdb04752bbc536007a920d24acb045561c26",
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ] {
            assert_eq!(validate_address(address), Ok(()), "{address}");
        }
    }

    #[test]
    fn an_all_lowercase_address_carries_no_checksum_and_passes() {
        assert_eq!(
            validate_address("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed"),
            Ok(())
        );
    }

    #[test]
    fn an_address_with_uppercase_letters_must_match_its_checksum() {
        assert_eq!(
            validate_address("0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed").unwrap_err(),
            "address 0x5AAeb6053F3E94C9b9A09f33669435E7Ef1BeAed does not match its EIP-55 checksum"
        );
        assert_eq!(
            validate_address("0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED").unwrap_err(),
            "address 0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED does not match its EIP-55 checksum"
        );
    }

    #[test]
    fn an_address_that_is_not_0x_and_40_hex_digits_fails_by_shape() {
        for address in [
            "",
            "5aaeb6053f3e94c9b9a09f33669435e7ef1beaed",
            "0X5aaeb6053f3e94c9b9a09f33669435e7ef1beaed",
            " 0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed",
            "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beae",
            "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaedd",
            "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaeg",
        ] {
            assert_eq!(
                validate_address(address).unwrap_err(),
                format!("address {address:?} is not 0x and 40 hex digits")
            );
        }
    }
}
