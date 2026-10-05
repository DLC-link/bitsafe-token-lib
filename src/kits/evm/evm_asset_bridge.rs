//! The binding of one contract, `BitSafeEvmAssetBridge`, which serves one
//! asset per proxy on any EVM chain. It takes plain id strings and knows
//! nothing about Canton accounts.

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_sol_types::{SolCall, SolEvent, sol};

use crate::DamlDecimal;

pub use BitSafeEvmAssetBridge::Deposit;

/// One deployment of the bridge: where it lives and since when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvmBridge {
    /// The EIP-155 chain id: 1 for Ethereum mainnet, 11155111 for Sepolia.
    pub chain_id: u64,
    /// The proxy address that takes the deposits.
    pub proxy: Address,
    /// The block of the proxy's creation receipt. A caller that scans for
    /// its own `Deposit` events starts `eth_getLogs` here, not at genesis.
    pub deploy_block: u64,
}

// The deposit side of the bridge. `depositERC20` is declared for the next
// EVM asset; only `depositETH` has a wrapper.
sol! {
    interface BitSafeEvmAssetBridge {
        function depositETH(bytes calldata depositAccountId) external payable;
        function depositERC20(uint256 amount, bytes calldata depositAccountId) external;
        event Deposit(
            bytes32 indexed depositAccountIdHash,
            bytes depositAccountId,
            address indexed sender,
            uint256 amount
        );
    }
}

/// Every ETH deposit is a whole multiple of this many wei: the bridge's
/// `depositUnit()`, 10^(18 - 10) for 18 ETH decimals and 10 Daml decimals.
pub const DEPOSIT_UNIT_WEI: U256 = U256::from_limbs([100_000_000, 0, 0, 0]);

/// The topic0 of the bridge's `Deposit` event, for a log filter.
pub const DEPOSIT_EVENT_TOPIC: B256 = <Deposit as SolEvent>::SIGNATURE_HASH;

/// The calldata of `depositETH(bytes)` with the UTF-8 bytes of the id. The
/// attestors match a deposit on exactly these bytes.
pub(crate) fn encode_deposit_eth(deposit_id: &str) -> Bytes {
    BitSafeEvmAssetBridge::depositETHCall {
        depositAccountId: Bytes::copy_from_slice(deposit_id.as_bytes()),
    }
    .abi_encode()
    .into()
}

/// The topic1 that the bridge indexes a deposit under and that the
/// attestors filter on: keccak256 of the UTF-8 bytes of the id.
pub fn deposit_id_topic(deposit_id: &str) -> B256 {
    keccak256(deposit_id.as_bytes())
}

/// Converts wei to the Daml amount the deposit mints: wei / 10^18 ETH,
/// with 10 decimals.
///
/// # Errors
///
/// Fails when `wei` is not a whole multiple of 100000000, because such an
/// amount has no exact value with 10 decimals and the bridge refuses it.
/// Fails when the amount is too large for a Daml decimal.
pub fn wei_to_daml_decimal(wei: U256) -> Result<DamlDecimal, String> {
    if !(wei % DEPOSIT_UNIT_WEI).is_zero() {
        return Err(format!(
            "amount {wei} wei is not a whole multiple of {DEPOSIT_UNIT_WEI} wei"
        ));
    }
    // One unit is 10^-10 ETH, the smallest step of a Daml decimal.
    let units = wei / DEPOSIT_UNIT_WEI;
    let units_per_eth = U256::from(10_000_000_000_u64);
    let whole = units / units_per_eth;
    let fraction = (units % units_per_eth).to_string();
    let fraction = format!("{fraction:0>10}");
    let fraction = fraction.trim_end_matches('0');
    let text = if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    };
    let too_large = || format!("amount {wei} wei is too large for a Daml decimal");
    if whole >= U256::from(10_u64).pow(U256::from(28_u64)) {
        return Err(too_large());
    }
    let amount = DamlDecimal::parse(&text).map_err(|_| too_large())?;
    // The parser rounds input with more than about 28 significant digits.
    if daml_decimal_to_wei(amount) != Ok(wei) {
        return Err(too_large());
    }
    Ok(amount)
}

/// Converts a Daml amount of ETH to wei. Every Daml amount has at most 10
/// decimals, so the result is exact.
///
/// # Errors
///
/// Fails when the amount is negative.
pub fn daml_decimal_to_wei(amount: DamlDecimal) -> Result<U256, String> {
    if amount < DamlDecimal::ZERO {
        return Err(format!("amount {amount} is negative"));
    }
    let value = amount.value();
    // A Daml decimal has at most 10 decimals, so the exponent is 8 to 18.
    let exponent = 18 - value.scale();
    Ok(U256::from(value.mantissa().unsigned_abs()) * U256::from(10_u64).pow(U256::from(exponent)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Produced by the attestors' own deposit-call encoder for the id `smoke`.
    const SMOKE_CALL: &str = "0xdeec7c8f00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000005736d6f6b65000000000000000000000000000000000000000000000000000000";
    const SMOKE_TOPIC: &str = "0x9f7b51b320b49b3e1ee04dd6b6b82190a86b134784c03404ac63c7b40be5b431";
    /// An id with the shape of a Canton contract id, 138 characters.
    const CID: &str = "00a1b2c3d4e5f60718a1b2c3d4e5f60718a1b2c3d4e5f60718a1b2c3d4e5f60718ca1212200d3b6e1f9a8c7b5e0d3b6e1f9a8c7b5e0d3b6e1f9a8c7b5e0d3b6e1f9a8c7b5e";
    const CID_TOPIC: &str = "0xb0d0b6f6dfb8154461736748a59822a1511768410776754bd3db3ffb726b6801";

    fn wei(text: &str) -> U256 {
        text.parse().unwrap()
    }

    fn decimal(text: &str) -> DamlDecimal {
        DamlDecimal::parse(text).unwrap()
    }

    #[test]
    fn the_selectors_and_the_event_topic_match_the_deployed_bridge() {
        assert_eq!(
            BitSafeEvmAssetBridge::depositETHCall::SELECTOR,
            [0xde, 0xec, 0x7c, 0x8f]
        );
        assert_eq!(
            BitSafeEvmAssetBridge::depositERC20Call::SELECTOR,
            [0x7b, 0xd2, 0x10, 0xe5]
        );
        assert_eq!(
            DEPOSIT_EVENT_TOPIC.to_string(),
            "0xacaea730ea841fae3326f72a7ff746b6e06c646840526bcaf5dabee5fd887ef8"
        );
    }

    #[test]
    fn encode_deposit_eth_matches_the_attestor_stack_encoder() {
        assert_eq!(encode_deposit_eth("smoke").to_string(), SMOKE_CALL);
    }

    #[test]
    fn deposit_id_topic_is_keccak_of_the_utf8_id() {
        assert_eq!(deposit_id_topic("smoke").to_string(), SMOKE_TOPIC);
        assert_eq!(deposit_id_topic(CID).to_string(), CID_TOPIC);
    }

    #[test]
    fn the_deposit_unit_is_1e8_wei() {
        assert_eq!(DEPOSIT_UNIT_WEI, wei("100000000"));
    }

    #[test]
    fn wei_converts_to_a_daml_decimal_with_no_trailing_zeros() {
        let cases = [
            ("1000000000000000000", "1"),
            ("5000000000000000", "0.005"),
            ("100000000", "0.0000000001"),
            ("12345678900000000", "0.0123456789"),
            ("101000000000000000000", "101"),
            ("0", "0"),
        ];
        for (amount, expected) in cases {
            let converted = wei_to_daml_decimal(wei(amount)).unwrap();
            assert_eq!(converted.to_string(), expected, "{amount}");
        }
    }

    #[test]
    fn wei_that_is_not_a_whole_unit_fails() {
        assert_eq!(
            wei_to_daml_decimal(wei("100000001")).unwrap_err(),
            "amount 100000001 wei is not a whole multiple of 100000000 wei"
        );
    }

    #[test]
    fn wei_too_large_for_a_daml_decimal_fails() {
        let largest = U256::MAX / DEPOSIT_UNIT_WEI * DEPOSIT_UNIT_WEI;
        assert_eq!(
            wei_to_daml_decimal(largest).unwrap_err(),
            format!("amount {largest} wei is too large for a Daml decimal")
        );
    }

    #[test]
    fn wei_with_28_whole_eth_digits_or_more_fails() {
        // 5 * 10^28 ETH, in wei.
        let amount = wei("50000000000000000000000000000000000000000000000");
        assert_eq!(
            wei_to_daml_decimal(amount).unwrap_err(),
            format!("amount {amount} wei is too large for a Daml decimal")
        );
    }

    #[test]
    fn wei_that_a_daml_decimal_would_round_fails() {
        // 10^19 ETH plus 0.0123456789 ETH, in wei: 30 significant digits.
        let amount = wei("10000000000000000000012345678900000000");
        assert_eq!(
            wei_to_daml_decimal(amount).unwrap_err(),
            format!("amount {amount} wei is too large for a Daml decimal")
        );
    }

    #[test]
    fn a_daml_decimal_converts_to_wei_exactly() {
        assert_eq!(
            daml_decimal_to_wei(decimal("1")).unwrap(),
            wei("1000000000000000000")
        );
        assert_eq!(
            daml_decimal_to_wei(decimal("0.0000000001")).unwrap(),
            wei("100000000")
        );
        assert_eq!(
            daml_decimal_to_wei(decimal("1.0000000000")).unwrap(),
            wei("1000000000000000000")
        );
        let amount = wei("12345678900000000");
        assert_eq!(
            daml_decimal_to_wei(wei_to_daml_decimal(amount).unwrap()).unwrap(),
            amount
        );
    }

    #[test]
    fn a_negative_daml_decimal_does_not_convert_to_wei() {
        assert_eq!(
            daml_decimal_to_wei(decimal("-1")).unwrap_err(),
            "amount -1 is negative"
        );
    }
}
