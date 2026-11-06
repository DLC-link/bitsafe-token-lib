//! The mint side of the assets that the EVM asset bridge serves: BETH and
//! CHYPE. Each asset re-exports the two structs in its `mint` module and
//! wraps the two builders with its own `Network` lookup, so an account of
//! one asset cannot reach another asset's bridge.

use alloy_primitives::{Address, Bytes, U256};

use crate::{
    flows::canton_bridge_v1::{check_registrar, models::DepositAccount},
    kits::evm::evm_asset_bridge::{
        DEPOSIT_UNIT_WEI, EvmBridge, encode_deposit_eth, wei_to_daml_decimal,
    },
};

/// The fields of a `depositETH` transaction. The caller signs and sends it
/// with its own EVM wallet; this crate sends nothing. The bridge names its
/// native-coin deposit `depositETH` on every chain, HyperEVM too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositCall {
    /// The EIP-155 chain to send on, such as 1 for Ethereum mainnet or 999
    /// for HyperEVM.
    pub chain_id: u64,
    /// The asset's bridge proxy on that chain.
    pub to: Address,
    /// The native coin to deposit, in wei: the amount the caller passed.
    pub value: U256,
    /// `depositETH(bytes)` with the UTF-8 bytes of the account id.
    pub data: Bytes,
    /// The account id that `data` carries, for the caller's records.
    pub deposit_id: String,
}

/// Where a deposit for an account goes, for a front end that builds the
/// transaction itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositTarget {
    /// The EIP-155 chain to send on, such as 1 for Ethereum mainnet or 999
    /// for HyperEVM.
    pub chain_id: u64,
    /// The asset's bridge proxy on that chain.
    pub to: Address,
    /// The account id that the attestors match a deposit on.
    pub deposit_id: String,
    /// The `depositETH` argument: the UTF-8 bytes of `deposit_id`.
    pub deposit_id_bytes: Bytes,
    /// Every deposit must be a whole multiple of this many wei.
    pub deposit_unit_wei: U256,
}

/// The `depositETH(bytes)` transaction that mints to the account, for the
/// asset's registrar and bridge on one network.
pub(crate) fn deposit_call<A>(
    registrar: &str,
    bridge: EvmBridge,
    account: &DepositAccount<A>,
    amount_wei: U256,
) -> Result<DepositCall, String> {
    let target = deposit_target(registrar, bridge, account)?;
    if amount_wei.is_zero() {
        return Err("Deposit amount must be greater than 0".to_string());
    }
    account.check_amount(wei_to_daml_decimal(amount_wei)?)?;
    Ok(DepositCall {
        chain_id: target.chain_id,
        to: target.to,
        value: amount_wei,
        data: encode_deposit_eth(&target.deposit_id),
        deposit_id: target.deposit_id,
    })
}

/// The chain, the proxy and the deposit id of the account, for the asset's
/// registrar and bridge on one network.
pub(crate) fn deposit_target<A>(
    registrar: &str,
    bridge: EvmBridge,
    account: &DepositAccount<A>,
) -> Result<DepositTarget, String> {
    check_registrar(&account.registrar, registrar)?;
    let deposit_id = account.account_id();
    if deposit_id.is_empty() {
        return Err("empty deposit id".to_string());
    }
    Ok(DepositTarget {
        chain_id: bridge.chain_id,
        to: bridge.proxy,
        deposit_id: deposit_id.to_string(),
        deposit_id_bytes: Bytes::copy_from_slice(deposit_id.as_bytes()),
        deposit_unit_wei: DEPOSIT_UNIT_WEI,
    })
}
