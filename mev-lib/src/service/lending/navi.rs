use crate::{
    config::NaviConfig,
    constant,
    service::{db_service, lending::LendingService},
    types::{CalcHFResult, U256},
    utils::{self, ptb::PTBHelper},
};
use bigdecimal::BigDecimal;
use db::models::{self, lending_market};
use db::repositories::{CoinRepository, LendingMarketRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use rust_decimal::{Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::{
    collections::HashSet, fmt::Display, fs::File, io::Write, path::Path, str::FromStr, sync::Arc,
};
use sui_json_rpc_types::{SuiObjectDataOptions, SuiParsedData};
use sui_sdk::SuiClient;
use sui_types::{
    base_types::{ObjectID, SequenceNumber, SuiAddress},
    event::Event,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Command, ObjectArg, TransactionData, TransactionKind},
    Identifier,
};
use tokio::{
    self,
    time::{Duration, Instant},
};
use toml;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct MarketConfig {
    pub asset_id: u8,
    pub ltv: String,
    pub liquidation_threshold: String,
    pub liquidation_ratio: String,
    pub liquidation_bonus: String,
    pub supply_index: String,
    pub borrow_index: String,
    pub treasury_factor: String,
}

pub struct NaviService {
    platform: String,
    config: Arc<NaviConfig>,
    client: Arc<SuiClient>,
    market_repo: Arc<dyn LendingMarketRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    db_service: Arc<db_service::lending::LendingService>,
    ptb_helper: Arc<PTBHelper>,
}

impl NaviService {
    pub fn new(
        config: Arc<NaviConfig>,
        client: Arc<SuiClient>,
        market_repo: Arc<dyn LendingMarketRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        db_service: Arc<db_service::lending::LendingService>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        NaviService {
            platform: constant::NAVI_LENDING.to_string(),
            config,
            client,
            market_repo,
            coin_repo,
            db_service,
            ptb_helper,
        }
    }
}

#[async_trait]
impl LendingService for NaviService {
    /// Fetches the borrower portfolio from onchain data.
    ///
    async fn fetch_borrower_portfolio(
        &self,
        borrower: String,
        obligation_id: Option<String>,
    ) -> Result<(
        Vec<crate::types::UserDeposit>,
        Vec<crate::types::UserBorrow>,
    )> {
        let start = Instant::now();
        // query user assets
        let mut ptb = ProgrammableTransactionBuilder::new();

        let storage_arg = ptb.obj(
            self.ptb_helper
                .build_shared_obj_arg(&self.config.storage_id, false)
                .await?,
        )?;

        let user_arg = ptb.pure(SuiAddress::from_str(&borrower)?)?;

        ptb.command(Command::move_call(
            ObjectID::from_str(&self.config.package_id)?,
            Identifier::new("storage")?,
            Identifier::new("get_user_assets")?,
            vec![],
            vec![storage_arg, user_arg],
        ));

        let builder = ptb.finish();

        let tx = TransactionKind::ProgrammableTransaction(builder);

        let response = self
            .client
            .read_api()
            .dev_inspect_transaction_block(SuiAddress::default(), tx, None, None, None)
            .await?;

        let values = response.results.ok_or(anyhow!(
            "Failed to get return values from dev_inspect_transaction_block"
        ))?;

        let return_values = &values
            .first()
            .ok_or(anyhow!("Failed to get collaterals from return values"))?
            .return_values;

        let collaterals = return_values
            .first()
            .ok_or(anyhow!("Failed to get collaterals from return values"))?;

        let mut collaterals = bcs::from_bytes::<Vec<u8>>(&collaterals.0)?;

        let loans = return_values
            .get(1)
            .ok_or(anyhow!("Failed to get loans from return values"))?;

        let loans = bcs::from_bytes::<Vec<u8>>(&loans.0)?;

        let assets: HashSet<u8> = collaterals.iter().chain(loans.iter()).cloned().collect();

        let elapsed = start.elapsed();
        info!(
            "get_user_assets response: collaterals assets {:?} loans assets {:?}, all assets {:?} in {:?}ms",
            collaterals, loans, assets, elapsed.as_millis()
        );

        // query user assets balances in parallel
        let user_balance_by_asset = stream::iter(assets)
            .map(|asset| {
                let borrower = borrower.clone();
                async move { self.fetch_borrower_balance(&borrower, asset).await }
            })
            .buffer_unordered(8)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let user_deposits = user_balance_by_asset
            .iter()
            .filter(|(deposit, _)| {
                Decimal::from_str(&deposit.amount).unwrap_or(Decimal::ZERO) > Decimal::ZERO
            })
            .map(|(deposit, _)| deposit.clone())
            .collect::<Vec<_>>();

        let user_borrows = user_balance_by_asset
            .iter()
            .filter(|(_, borrow)| {
                Decimal::from_str(&borrow.amount).unwrap_or(Decimal::ZERO) > Decimal::ZERO
            })
            .map(|(_, borrow)| borrow.clone())
            .collect::<Vec<_>>();

        Ok((user_deposits, user_borrows))
    }

    /// Fetch user deposit for an asset from onchain data.
    ///
    async fn fetch_user_deposit(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserDeposit> {
        let (user_deposit, _) = self
            .fetch_borrower_balance(
                &borrower,
                asset_id.ok_or_else(|| anyhow!("Asset ID is required to fetch user deposit"))?,
            )
            .await?;

        Ok(user_deposit)
    }

    /// Fetch user borrow for an asset from onchain data.
    ///
    async fn fetch_user_borrow(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserBorrow> {
        let (_, user_borrow) = self
            .fetch_borrower_balance(
                &borrower,
                asset_id.ok_or_else(|| anyhow!("Asset ID is required to fetch user borrow"))?,
            )
            .await?;

        Ok(user_borrow)
    }
}

impl NaviService {
    async fn fetch_borrower_balance(
        &self,
        borrower: &str,
        asset_id: u8,
    ) -> Result<(crate::types::UserDeposit, crate::types::UserBorrow)> {
        let start = Instant::now();

        // Process user asset logic here
        let mut ptb = ProgrammableTransactionBuilder::new();

        let storage_arg = ptb.obj(
            self.ptb_helper
                .build_shared_obj_arg(&self.config.storage_id, true)
                .await?,
        )?;

        let asset_arg = ptb.pure::<u8>(asset_id)?;
        let user_arg = ptb.pure(SuiAddress::from_str(borrower)?)?;

        ptb.command(Command::move_call(
            ObjectID::from_str(&self.config.package_id)?,
            Identifier::new("storage")?,
            Identifier::new("get_user_balance")?,
            vec![],
            vec![storage_arg, asset_arg, user_arg],
        ));

        let builder = ptb.finish();

        let tx = TransactionKind::ProgrammableTransaction(builder);

        let response = self
            .client
            .read_api()
            .dev_inspect_transaction_block(SuiAddress::default(), tx, None, None, None)
            .await?;

        let results = response.results.ok_or(anyhow!(
            "Failed to get return values from dev_inspect_transaction_block"
        ))?;

        let return_values = &results
            .first()
            .ok_or(anyhow!("Failed to get collaterals from return values"))?
            .return_values;

        let supply = return_values
            .first()
            .ok_or(anyhow!("Failed to get collaterals from return values"))?;
        let supply = bcs::from_bytes::<U256>(&supply.0)?;

        let borrow = return_values
            .get(1)
            .ok_or(anyhow!("Failed to get loans from return values"))?;
        let borrow = bcs::from_bytes::<U256>(&borrow.0)?;

        let elapsed = start.elapsed();
        info!(
            "get_user_balance response: asset {:?} supply {:?} borrow {:?} in {:?}ms",
            asset_id,
            supply,
            borrow,
            elapsed.as_millis()
        );

        let market = self
            .db_service
            .find_market_by_platform_and_asset_id(&self.platform, asset_id as i32)?;

        // insert user deposit and borrow
        let user_deposit = crate::types::UserDeposit {
            platform: self.platform.clone(),
            borrower: borrower.to_string(),
            coin_type: market.coin_type.clone(),
            amount: supply.to_string(),
            obligation_id: None,
        };

        let user_borrow = crate::types::UserBorrow {
            platform: self.platform.clone(),
            borrower: borrower.to_string(),
            coin_type: market.coin_type.clone(),
            amount: borrow.to_string(),
            obligation_id: None,
            debt_borrow_index: None,
        };

        info!(
            "Assets {:?} user deposit {:?} user borrow {:?}",
            asset_id, user_deposit, user_borrow
        );

        Ok((user_deposit, user_borrow))
    }
}
