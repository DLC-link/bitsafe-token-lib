//! The CBTC localnet suite: one test that runs the phases in order over one
//! fixture, because each phase uses the state the previous one left.

use crate::{
    Network,
    credentials::{
        CredentialOffer, ListCredentialOffersParams, ListCredentialsParams, UserCredential,
        list_credential_offers, list_credentials,
    },
    flows::canton_bridge_v1::credentials::{minter_credential_cids, minter_credential_offers},
    localnet::fixture::{Fixture, GOVERNANCE_RULES, INSTRUMENT_CONFIGURATION},
    tokens::cbtc::{self, Cbtc},
};

#[tokio::test]
#[ignore = "needs the localnet sandbox; see src/localnet/mod.rs"]
async fn localnet_cbtc() {
    let fixture = Fixture::new().await.expect("fixture");
    let rules = fixture
        .ledger
        .active(&fixture.registrar, GOVERNANCE_RULES)
        .await
        .expect("governance rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].created_event.contract_id, fixture.governance_rules);
    let instrument_configs = fixture
        .ledger
        .active(&fixture.registrar, INSTRUMENT_CONFIGURATION)
        .await
        .expect("instrument configurations");
    assert_eq!(instrument_configs.len(), 1);
    let rules = fixture.account_rules::<Cbtc>();
    assert_eq!(
        rules.da_rules.contract_id,
        fixture.deposit_rules.contract_id
    );
    assert!(!rules.da_rules.created_event_blob.is_empty());
    let contracts = fixture.token_standard_contracts::<Cbtc>();
    assert!(contracts.issuer_credential.is_some());
    assert_eq!(fixture.instrument().id, "CBTC");

    // The phases after this one use the cids.
    let _minter_cids = credentials_phase(&fixture).await;
}

/// The credentials phase. The registrar offers the test user the CBTC Minter
/// claim, and the user accepts it. Returns the user's Minter credential cids.
async fn credentials_phase(fixture: &Fixture) -> Vec<String> {
    let registrar = fixture.registrar.as_str();

    // Before the offer, the user holds no Minter credential and no offer.
    let credentials = user_credentials(fixture).await;
    assert!(minter_credential_cids::<Cbtc>(registrar, &credentials).is_empty());
    let offers = user_offers(fixture).await;
    assert!(minter_credential_offers::<Cbtc>(registrar, &offers).is_empty());

    let offer = fixture
        .offer_minter_credential()
        .await
        .expect("offer the minter credential");
    let offers = user_offers(fixture).await;
    let pending = minter_credential_offers::<Cbtc>(registrar, &offers);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, offer);

    fixture
        .accept_offer(&offer)
        .await
        .expect("accept the minter credential");
    let credentials = user_credentials(fixture).await;
    let minter_cids = minter_credential_cids::<Cbtc>(registrar, &credentials);
    assert_eq!(minter_cids.len(), 1);

    // The public wrapper checks the network's registrar, which is not the
    // sandbox registrar.
    assert!(cbtc::minter_credential_cids(Network::Devnet, &credentials).is_empty());
    minter_cids
}

/// The credentials the test user holds.
async fn user_credentials(fixture: &Fixture) -> Vec<UserCredential> {
    list_credentials(ListCredentialsParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credentials")
}

/// The credential offers the test user holds.
async fn user_offers(fixture: &Fixture) -> Vec<CredentialOffer> {
    list_credential_offers(ListCredentialOffersParams {
        ledger_host: fixture.ledger.host.clone(),
        party: fixture.user.clone(),
        access_token: fixture.ledger.token.clone(),
    })
    .await
    .expect("list the user's credential offers")
}
