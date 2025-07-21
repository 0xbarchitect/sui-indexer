pub mod navi;
pub mod scallop;
pub mod suilend;

use db::models;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

#[async_trait]
pub trait LendingService {
    /// Fetch the borrower poftfolio from on-chain data.
    /// Then save to the database.
    ///
    async fn fetch_borrower_portfolio(
        &self,
        borrower: String,
        obligation_id: Option<String>,
    ) -> Result<(
        Vec<crate::types::UserDeposit>,
        Vec<crate::types::UserBorrow>,
    )>;

    /// Fetch the borrower deposit from on-chain data.
    ///
    async fn fetch_user_deposit(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserDeposit>;

    /// Fetch the borrower borrow from on-chain data.
    ///
    async fn fetch_user_borrow(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserBorrow>;

    async fn lookup_borrower_hf_onchain(&self, borrower: String) -> Result<()> {
        Err(anyhow!(
            "Health factor fetching is not supported for this platform"
        ))
    }

    /// Fetches the lending market config for a specific platform.
    /// The lending market includes, but is not limited to:
    /// - Assets
    /// - LTV
    /// - Liquidation threshold
    /// - Borrow weight
    /// - Liquidation ratio
    /// - Liquidation penalty
    /// - Liquidation fee
    ///
    /// Returns a `Result` indicating success or failure.
    async fn fetch_lending_market_info(
        &self,
        info_file: String,
    ) -> Result<Vec<models::lending_market::LendingMarket>> {
        Err(anyhow!(
            "Lending market info fetching is not supported for this platform"
        ))
    }

    async fn fetch_reserve_info(&self, coin_type: String) -> Result<crate::types::LendingMarket> {
        Err(anyhow!(
            "Reserve info fetching is not supported for this platform"
        ))
    }

    async fn fetch_reserve_infos(
        &self,
        coin_types: Vec<String>,
    ) -> Result<Vec<crate::types::LendingMarket>> {
        Err(anyhow!(
            "Reserve info fetching is not supported for this platform"
        ))
    }

    async fn find_obligation_id_from_address(&self, borrower: &str) -> Result<String> {
        Err(anyhow!(
            "Finding obligation ID from address is not supported for this platform"
        ))
    }
}
