//! Local derivation of the CBTC deposit address, so the lib does not have to
//! trust the BitSafe API for it.
//!
//! The derivation mirrors `generate_taproot_output_info` in the attestor
//! (`dlc-attestor-stack`, `attestor/src/attestor/cbtc_bitcoin.rs`). Its inputs
//! are public: the attestor group key, the account id and the network.

use std::str::FromStr;

use bitcoin::{
    Address, KnownHrp, NetworkKind,
    bip32::{ChainCode, ChildNumber, Fingerprint, Xpub},
    hashes::{Hash, sha256},
    opcodes::all::OP_CHECKSIG,
    script::Builder,
    secp256k1::{PublicKey, Secp256k1},
    taproot::TaprootBuilder,
};

use crate::Network;

/// The BIP-341 "nothing up my sleeve" point. Nobody knows its private key,
/// so the key path of the deposit output cannot be spent.
const UNSPENDABLE_PUBLIC_KEY: &str =
    "0250929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// The attestor group key on each network, at the path the attestors sign
/// with (m/0/0 below the group master key). The attestors publish the same
/// string at `/cbtc/v1/address-calculation-data`.
///
/// The value changes when the attestors run a new DKG. This lib then refuses
/// every deposit address until a release pins the new key, so a merchant
/// never funds an address that the new group does not control.
const fn attestor_group_xpub(network: Network) -> &'static str {
    match network {
        Network::Devnet => {
            "tpubDGuLmmaULQdsPE27MNpQnv2jw1WgRmJWtaW9GT5R4syVLv986PH9CWkwrbdwjWmYKCAphoEZMpTeXgLjtN1AxbxyCKT8d9Sd5L4bCR2T6cf"
        }
        Network::Testnet => {
            "tpubDFuVbfP8s8TgSRJiS5XWf6XT3HioCfY42aDYzNKxyXtmMzKwLxDV8V5cgURXnb6zMsdTWXTLgpVY7SmjWEWa3FQezNuVwdVGJquL1XaQc6p"
        }
        Network::Mainnet => {
            "xpub6GRcASRzGcNLhsaTsX28aV1JmNZSdUFKHsXgSjwX4ykk8X3j58gznGf73mBe1k35A69K7JNZfwZhmQHjZGd8f5ine2ztkbW3yiayRHFRFKL"
        }
    }
}

/// The Bitcoin network that the attestors of each BitSafe network watch.
fn bitcoin_network(network: Network) -> bitcoin::Network {
    match network {
        Network::Devnet => bitcoin::Network::Regtest,
        Network::Testnet => bitcoin::Network::Testnet,
        Network::Mainnet => bitcoin::Network::Bitcoin,
    }
}

/// The address the attestors derive for `account_id` on `network`.
pub(super) fn derive(network: Network, account_id: &str) -> Result<String, String> {
    let secp = Secp256k1::verification_only();
    let btc_network = bitcoin_network(network);

    let group_key = Xpub::from_str(attestor_group_xpub(network))
        .map_err(|e| format!("pinned attestor key for {network} does not parse: {e}"))?
        .to_x_only_pub();
    let leaf = Builder::new()
        .push_x_only_key(&group_key)
        .push_opcode(OP_CHECKSIG)
        .into_script();

    // Only the public key and the chain code reach the derived key. The
    // attestor's depth, fingerprint and child number are metadata.
    let unspendable = Xpub {
        network: NetworkKind::from(btc_network),
        depth: 0,
        parent_fingerprint: Fingerprint::default(),
        child_number: ChildNumber::Normal { index: 0 },
        public_key: PublicKey::from_str(UNSPENDABLE_PUBLIC_KEY).map_err(|e| e.to_string())?,
        chain_code: ChainCode::from(sha256::Hash::hash(account_id.as_bytes()).to_byte_array()),
    };
    let internal_key = unspendable
        .derive_pub(
            &secp,
            &[
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ],
        )
        .map_err(|e| format!("cannot derive the internal key: {e}"))?
        .to_x_only_pub();

    let spend_info = TaprootBuilder::new()
        .add_leaf(0, leaf)
        .map_err(|e| e.to_string())?
        .finalize(&secp, internal_key)
        .map_err(|_| "cannot build the taproot tree".to_string())?;

    Ok(Address::p2tr_tweaked(spend_info.output_key(), KnownHrp::from(btc_network)).to_string())
}

/// Returns the derived address when the API returned the same one.
///
/// # Errors
///
/// Fails when the two differ. Either the API is wrong, or the attestors use a
/// new group key that this release does not pin. BTC must not go to either
/// address until someone finds out which.
pub(super) fn verify(network: Network, account_id: &str, from_api: &str) -> Result<String, String> {
    let derived = derive(network, account_id)?;
    if derived == from_api {
        Ok(derived)
    } else {
        Err(format!(
            "deposit address mismatch for account {account_id}: the BitSafe API returned \
             {from_api}, but the attestor key that this release pins gives {derived}. \
             Do not send BTC to either address. Contact BitSafe."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each pair comes from the live attestors: the first account in
    /// `/cbtc/v1/address-calculation-data`, and `change`, the id of the
    /// attestors' own change output.
    const VECTORS: [(Network, &str, &str); 6] = [
        (
            Network::Devnet,
            "00caf337569fa06ea77cf4009c719877ea8bc9d83c53ddaa56724d34d97363d343ca111220c8bbe80c3b00aba1d575fc12341a0e600e41902301d262b3a8bb1265eedcf0c2",
            "bcrt1pmhnzze2rhh993uukf5v4jfjtr36gm26n8q9jq89n3jjpkq70gg5q54nexf",
        ),
        (
            Network::Devnet,
            "change",
            "bcrt1pp094pw4lfp3yfgxzmwh6d9myk4y62tm94pjfwswgm8m8f3m5pdnqysxhrx",
        ),
        (
            Network::Testnet,
            "003745b47b6d7a32c51cd10dbb20bfaa9422b19af7154de8a82cfc28258f49646bca1112206dccdf6bb2c0c141db186cd0544ca5a3581414e7622b4286c26eb54c6ae01327",
            "tb1pk9w6vfhtxpwl49l40550svpsdfx4wnn8qjjk77wy5yxmnhphchhs4uq87w",
        ),
        (
            Network::Testnet,
            "change",
            "tb1pkt7ruckrqsnpznv5er8wgm2xmqhzckrl6l7ku20z5k373npmxx8snlxtas",
        ),
        (
            Network::Mainnet,
            "000da3e972f71100337b6039740625861997098d4f4785c7cf0f88286a0208fe60ca111220c31237da239a2317724046f46cfab8d50076a2a0314874c773db3c43ddf910c2",
            "bc1p8zuluqy8pza2pt6sj7lnl5esjk9t2e4h2k6jna7dtmyhhxd8uv0qp3tw9x",
        ),
        (
            Network::Mainnet,
            "change",
            "bc1ppp6ft7ragayvsuf8jsq2x6c5m083gpj6kz3rwfcm0efvcgsgkazsdywnne",
        ),
    ];

    #[test]
    fn derives_the_address_the_live_attestors_give() {
        for (network, id, address) in VECTORS {
            assert_eq!(derive(network, id).unwrap(), address, "{network} {id}");
        }
    }

    #[test]
    fn verify_returns_the_address_when_the_api_agrees() {
        for (network, id, address) in VECTORS {
            assert_eq!(verify(network, id, address).unwrap(), address);
        }
    }

    #[test]
    fn verify_refuses_an_address_the_attestors_do_not_control() {
        let (network, id, address) = VECTORS[4];
        let swapped = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
        let error = verify(network, id, swapped).unwrap_err();
        assert!(error.contains(swapped), "{error}");
        assert!(error.contains(address), "{error}");
        assert!(error.contains("Do not send BTC"), "{error}");
    }

    /// An address for the right account on another network is still wrong.
    #[test]
    fn verify_refuses_the_same_account_on_another_network() {
        let (_, id, mainnet_address) = VECTORS[4];
        assert!(verify(Network::Testnet, id, mainnet_address).is_err());
    }

    #[test]
    fn each_network_pins_its_own_key() {
        let keys: Vec<_> = Network::ALL
            .into_iter()
            .map(|network| {
                Xpub::from_str(attestor_group_xpub(network))
                    .unwrap()
                    .to_x_only_pub()
            })
            .collect();
        assert_ne!(keys[0], keys[1]);
        assert_ne!(keys[1], keys[2]);
        assert_ne!(keys[0], keys[2]);
    }

    /// Checks every account address and the pinned key against the live
    /// attestors. Run it before a release, and after any attestor DKG:
    /// `cargo test --lib -- --ignored pinned_keys_match_the_live_attestors`.
    #[tokio::test]
    #[ignore = "reads the live BitSafe API"]
    async fn pinned_keys_match_the_live_attestors() {
        #[derive(serde::Deserialize)]
        struct Data {
            chains: Vec<Chain>,
        }
        #[derive(serde::Deserialize)]
        struct Chain {
            xpub: String,
            addresses: Vec<Entry>,
        }
        #[derive(serde::Deserialize)]
        struct Entry {
            id: String,
            address_for_verification: String,
        }

        for network in Network::ALL {
            let url = format!(
                "{}/cbtc/v1/address-calculation-data",
                network.bitsafe_api_url()
            );
            let data: Data = reqwest::get(&url).await.unwrap().json().await.unwrap();
            let [chain] = data.chains.as_slice() else {
                panic!("{network}: expected one chain");
            };
            assert_eq!(chain.xpub, attestor_group_xpub(network), "{network}");
            for entry in &chain.addresses {
                assert_eq!(
                    derive(network, &entry.id).unwrap(),
                    entry.address_for_verification,
                    "{network} {}",
                    entry.id
                );
            }
        }
    }
}
