//! The Minter credential helpers of the family. The account and burn choices
//! require a credential with the asset's Minter claim, issued by the
//! registrar; the attestors check the same issuer.

use token::credentials::{Claim, UserCredential};

use crate::flows::canton_bridge_v1::CantonBridgeV1;

/// True when a claim list holds the asset's Minter claim.
fn has_minter_claim<A: CantonBridgeV1>(claims: &[Claim]) -> bool {
    let (property, value) = A::MINTER_CLAIM;
    claims
        .iter()
        .any(|claim| claim.property == property && claim.value == value)
}

/// The cids of the credentials that carry the asset's Minter claim and come
/// from the registrar. The account and burn choices take these.
pub(crate) fn minter_credential_cids<A: CantonBridgeV1>(
    registrar: &str,
    credentials: &[UserCredential],
) -> Vec<String> {
    credentials
        .iter()
        .filter(|credential| {
            credential.issuer == registrar && has_minter_claim::<A>(&credential.claims)
        })
        .map(|credential| credential.contract_id.clone())
        .collect()
}

/// The pending offers that would grant the asset's Minter role from the
/// registrar. A front end shows these before the party accepts one.
pub(crate) fn minter_credential_offers<'a, A: CantonBridgeV1>(
    registrar: &str,
    offers: &'a [token::credentials::CredentialOffer],
) -> Vec<&'a token::credentials::CredentialOffer> {
    offers
        .iter()
        .filter(|offer| offer.issuer == registrar && has_minter_claim::<A>(&offer.claims))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::cbtc::Cbtc;

    fn credential(cid: &str, issuer: &str, property: &str, value: &str) -> UserCredential {
        UserCredential {
            contract_id: cid.to_string(),
            template_id: "pkg:Credential".to_string(),
            issuer: issuer.to_string(),
            holder: "alice".to_string(),
            id: cid.to_string(),
            description: String::new(),
            claims: vec![Claim {
                subject: "alice".to_string(),
                property: property.to_string(),
                value: value.to_string(),
            }],
        }
    }

    #[test]
    fn minter_credential_cids_keep_the_asset_claim_from_the_registrar_only() {
        let credentials = vec![
            credential("00keep", "cbtc-network::1220", "hasCBTCRole", "Minter"),
            credential("00issuer", "someone-else::1220", "hasCBTCRole", "Minter"),
            credential("00role", "cbtc-network::1220", "hasCBTCRole", "Holder"),
            credential("00asset", "cbtc-network::1220", "hasBETHRole", "Minter"),
        ];
        assert_eq!(
            minter_credential_cids::<Cbtc>("cbtc-network::1220", &credentials),
            vec!["00keep".to_string()]
        );
    }

    #[test]
    fn minter_credential_offers_keep_the_registrar_offer_with_the_asset_claim() {
        use token::credentials::CredentialOffer;
        let offer = |cid: &str, issuer: &str, property: &str| CredentialOffer {
            contract_id: cid.to_string(),
            template_id: "pkg:CredentialOffer".to_string(),
            created_event_blob: String::new(),
            issuer: issuer.to_string(),
            holder: "alice".to_string(),
            id: cid.to_string(),
            description: String::new(),
            claims: vec![Claim {
                subject: "alice".to_string(),
                property: property.to_string(),
                value: "Minter".to_string(),
            }],
        };
        let offers = vec![
            offer("00keep", "cbtc-network::1220", "hasCBTCRole"),
            offer("00issuer", "x::1220", "hasCBTCRole"),
            offer("00asset", "cbtc-network::1220", "hasBETHRole"),
        ];
        let kept: Vec<&str> = minter_credential_offers::<Cbtc>("cbtc-network::1220", &offers)
            .into_iter()
            .map(|o| o.contract_id.as_str())
            .collect();
        assert_eq!(kept, vec!["00keep"]);
    }
}
