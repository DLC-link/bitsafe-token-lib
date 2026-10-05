//! The binding of one contract, `BitSafeEvmAssetBridge`, which serves one
//! asset per proxy on any EVM chain. It takes plain id strings and knows
//! nothing about Canton accounts.

use alloy_primitives::Address;

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
