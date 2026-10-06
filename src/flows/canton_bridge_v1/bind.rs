//! The macro that binds the family to one asset.

/// Emits a private `family` module with the family's flows bound to one
/// asset: one-line public wrappers that resolve `Network` to the registrar
/// and the API URL, a type alias per model, and the parameter structs.
///
/// The asset module must define `registrar(network: Network) -> &'static str`.
/// It then re-exports what it exposes:
///
/// ```ignore
/// bind_canton_bridge_v1!(Cbtc);
/// pub mod mint {
///     pub use super::family::mint::*;
///     // the asset's own mint helpers
/// }
/// pub mod redeem { pub use super::family::redeem::*; }
/// pub use family::{minter_credential_cids, minter_credential_offers};
/// ```
///
/// The wrappers hold no logic, so every asset's wrappers are the same text.
macro_rules! bind_canton_bridge_v1 {
    ($asset:ident) => {
        mod family {
            use super::$asset;
            use $crate::Network;
            use $crate::flows::canton_bridge_v1 as f;

            pub mod mint {
                use super::{Network, f, $asset};

                pub use f::deposit::ListDepositAccountsParams;
                pub use $crate::kits::bitsafe_api::ContractInfo;
                pub use $crate::kits::canton::Limits;

                /// A deposit account of this asset. `account_id()` returns the id that the
                /// attestors key on, and `check_amount(amount)` tests an amount against the
                /// account's limits.
                pub type DepositAccount = f::models::DepositAccount<$asset>;
                /// This asset's account rules contracts.
                pub type AccountContractRuleSet =
                    $crate::kits::bitsafe_api::AccountContractRuleSet<$asset>;
                /// The parameters for creating a deposit account of this asset.
                pub type CreateDepositAccountParams =
                    f::deposit::CreateDepositAccountParams<$asset>;

                /// Fetches the registrar-signed rules contracts from the BitSafe API.
                pub async fn get_account_contract_rules(
                    network: Network,
                ) -> Result<AccountContractRuleSet, String> {
                    f::deposit::get_account_contract_rules::<$asset>(network.bitsafe_api_url())
                        .await
                }

                /// Lists the party's live deposit accounts.
                pub async fn list_deposit_accounts(
                    params: ListDepositAccountsParams,
                ) -> Result<Vec<DepositAccount>, String> {
                    f::deposit::list_deposit_accounts::<$asset>(params).await
                }

                /// Creates a deposit account with the rules disclosed.
                pub async fn create_deposit_account(
                    params: CreateDepositAccountParams,
                ) -> Result<DepositAccount, String> {
                    f::deposit::create_deposit_account::<$asset>(params).await
                }

                /// Lists the party's deposit accounts and picks one by contract id.
                pub async fn find_deposit_account(
                    params: ListDepositAccountsParams,
                    contract_id: &str,
                ) -> Result<DepositAccount, String> {
                    f::deposit::find_deposit_account::<$asset>(params, contract_id).await
                }
            }

            pub mod redeem {
                use super::{Network, f, $asset};

                pub use super::mint::{
                    AccountContractRuleSet, ContractInfo, Limits, get_account_contract_rules,
                };
                pub use f::withdraw::{ListWithdrawAccountsParams, ListWithdrawRequestsParams};
                pub use $crate::kits::canton::{ListHoldingsParams, list_holdings};

                /// A withdraw account of this asset. `check_amount(amount)` tests an amount
                /// against the account's limits.
                pub type WithdrawAccount = f::models::WithdrawAccount<$asset>;
                /// A withdraw request of this asset.
                pub type WithdrawRequest = f::models::WithdrawRequest<$asset>;
                /// The parameters for creating a withdraw account of this asset.
                pub type CreateWithdrawAccountParams =
                    f::withdraw::CreateWithdrawAccountParams<$asset>;
                /// The parameters for a burn of this asset.
                pub type SubmitWithdrawParams<'a> = f::withdraw::SubmitWithdrawParams<'a, $asset>;

                /// Lists the party's live withdraw accounts.
                pub async fn list_withdraw_accounts(
                    params: ListWithdrawAccountsParams,
                ) -> Result<Vec<WithdrawAccount>, String> {
                    f::withdraw::list_withdraw_accounts::<$asset>(params).await
                }

                /// Checks the destination and creates a withdraw account.
                pub async fn create_withdraw_account(
                    params: CreateWithdrawAccountParams,
                ) -> Result<WithdrawAccount, String> {
                    f::withdraw::create_withdraw_account::<$asset>(params).await
                }

                /// Lists the party's withdraw accounts and picks one by contract id.
                pub async fn find_withdraw_account(
                    params: ListWithdrawAccountsParams,
                    contract_id: &str,
                ) -> Result<WithdrawAccount, String> {
                    f::withdraw::find_withdraw_account::<$asset>(params, contract_id).await
                }

                /// Burns the holdings into the withdraw account, after the
                /// local checks. Returns the account with its new pending balance.
                /// `params.amount` is a Daml decimal in the asset's own unit,
                /// BTC or ETH, not satoshi or wei.
                pub async fn submit_withdraw(
                    network: Network,
                    params: SubmitWithdrawParams<'_>,
                ) -> Result<WithdrawAccount, String> {
                    f::withdraw::submit_withdraw::<$asset>(
                        super::super::registrar(network),
                        network.bitsafe_api_url(),
                        params,
                    )
                    .await
                }

                /// Lists the registrar-created payout records.
                pub async fn list_withdraw_requests(
                    params: ListWithdrawRequestsParams,
                ) -> Result<Vec<WithdrawRequest>, String> {
                    f::withdraw::list_withdraw_requests::<$asset>(params).await
                }
            }

            /// The cids of the party's credentials with this asset's Minter
            /// claim from the network's registrar.
            pub fn minter_credential_cids(
                network: Network,
                credentials: &[$crate::credentials::UserCredential],
            ) -> Vec<String> {
                f::credentials::minter_credential_cids::<$asset>(
                    super::registrar(network),
                    credentials,
                )
            }

            /// The party's pending offers of this asset's Minter claim from
            /// the network's registrar.
            pub fn minter_credential_offers(
                network: Network,
                offers: &[$crate::credentials::CredentialOffer],
            ) -> Vec<&$crate::credentials::CredentialOffer> {
                f::credentials::minter_credential_offers::<$asset>(
                    super::registrar(network),
                    offers,
                )
            }
        }
    };
}

pub(crate) use bind_canton_bridge_v1;
