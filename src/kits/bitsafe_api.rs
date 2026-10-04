//! A typed GET client for the BitSafe API, and the response types its
//! current endpoints share. An asset with other endpoints declares its own
//! response types and calls `get_json` with them.

use std::marker::PhantomData;

use common::transfer::DisclosedContract;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::kits::canton;

/// A contract the BitSafe API hands out for disclosure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInfo {
    pub contract_id: String,
    pub template_id: String,
    pub created_event_blob: String,
}

impl ContractInfo {
    /// This contract as a disclosed contract for a submission.
    pub(crate) fn disclosed(&self) -> DisclosedContract {
        canton::disclosed(
            &self.contract_id,
            &self.template_id,
            &self.created_event_blob,
        )
    }
}

/// The registrar-signed rules contracts that create deposit and withdraw
/// accounts. The type parameter is the asset, so one asset's rules cannot
/// reach another asset's flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct AccountContractRuleSet<A> {
    /// The deposit account rules.
    pub da_rules: ContractInfo,
    /// The withdraw account rules.
    pub wa_rules: ContractInfo,
    #[serde(skip)]
    _asset: PhantomData<A>,
}

/// The utility contracts a burn discloses. The API's deprecated
/// `issuer_credential` field is not read, because no burn needs it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct TokenStandardContracts<A> {
    pub burn_mint_factory: ContractInfo,
    pub instrument_configuration: ContractInfo,
    #[serde(skip)]
    _asset: PhantomData<A>,
}

/// The URL of `path` under `api_url`, with one slash between them.
pub(crate) fn url(api_url: &str, path: &str) -> String {
    format!("{}/{}", api_url.trim_end_matches('/'), path)
}

/// Fetches `path` from the BitSafe API and parses the JSON body.
pub(crate) async fn get_json<T: DeserializeOwned>(api_url: &str, path: &str) -> Result<T, String> {
    let response = reqwest::get(url(api_url, path))
        .await
        .map_err(|e| format!("Failed to send request to Bitsafe API: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "Bitsafe API returned error status: {}",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Debug, Clone)]
    struct Asset;

    fn info(name: &str) -> serde_json::Value {
        json!({"template_id": format!("pkg:{name}"), "contract_id": format!("00{name}"), "created_event_blob": "blob"})
    }

    #[test]
    fn account_rules_deserialize_from_the_api_shape() {
        let rules: AccountContractRuleSet<Asset> =
            serde_json::from_value(json!({"da_rules": info("da"), "wa_rules": info("wa")}))
                .unwrap();
        assert_eq!(rules.da_rules.contract_id, "00da");
        assert_eq!(rules.wa_rules.template_id, "pkg:wa");
    }

    #[test]
    /// Today's API still sends the deprecated `issuer_credential`; the next
    /// version drops it. Both shapes must parse.
    fn token_standard_contracts_parse_with_and_without_the_deprecated_issuer_credential() {
        let with: TokenStandardContracts<Asset> = serde_json::from_value(json!({
            "burn_mint_factory": info("f"), "instrument_configuration": info("c"), "issuer_credential": info("i"),
        }))
        .unwrap();
        assert_eq!(with.burn_mint_factory.contract_id, "00f");
        let without: TokenStandardContracts<Asset> = serde_json::from_value(json!({
            "burn_mint_factory": info("f"), "instrument_configuration": info("c"),
        }))
        .unwrap();
        assert_eq!(without.instrument_configuration.contract_id, "00c");
    }

    #[test]
    fn a_contract_info_discloses_on_any_synchronizer() {
        let contract: ContractInfo = serde_json::from_value(info("da")).unwrap();
        let disclosed = contract.disclosed();
        assert_eq!(disclosed.contract_id, "00da");
        assert_eq!(disclosed.template_id.as_deref(), Some("pkg:da"));
        assert_eq!(disclosed.created_event_blob, "blob");
        assert_eq!(disclosed.synchronizer_id, "");
    }

    #[test]
    fn url_joins_with_exactly_one_slash() {
        assert_eq!(url("https://api.x", "cbtc/v1/a"), "https://api.x/cbtc/v1/a");
        assert_eq!(
            url("https://api.x/", "cbtc/v1/a"),
            "https://api.x/cbtc/v1/a"
        );
    }
}
