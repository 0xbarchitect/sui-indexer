use crate::{
    config::Config,
    constant,
    types::{BorrowerAsset, PythPrice},
    utils,
};
use db::models::{
    self,
    borrower::{Borrower, NewBorrower, UpdateBorrower},
    coin::{Coin, NewCoin, UpdateCoin},
    user_borrow, user_deposit,
};
use db::repositories::{
    BorrowerRepository, CoinRepository, MetricRepository, SharedObjectRepository,
    UserBorrowRepository, UserDepositRepository,
};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use rust_decimal::{prelude::*, Decimal};
use std::{collections::HashSet, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, trace, warn};

pub struct LendingService {
    config: Arc<Config>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    user_borrow_repo: Arc<dyn UserBorrowRepository + Send + Sync>,
    user_deposit_repo: Arc<dyn UserDepositRepository + Send + Sync>,
    borrower_repo: Arc<dyn BorrowerRepository + Send + Sync>,
    metric_repo: Arc<dyn MetricRepository + Send + Sync>,
    shared_object_repo: Arc<dyn SharedObjectRepository + Send + Sync>,
}

impl LendingService {
    pub fn new(
        config: Arc<Config>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        user_borrow_repo: Arc<dyn UserBorrowRepository + Send + Sync>,
        user_deposit_repo: Arc<dyn UserDepositRepository + Send + Sync>,
        borrower_repo: Arc<dyn BorrowerRepository + Send + Sync>,
        metric_repo: Arc<dyn MetricRepository + Send + Sync>,
        shared_object_repo: Arc<dyn SharedObjectRepository + Send + Sync>,
    ) -> Self {
        LendingService {
            config,
            coin_repo,
            user_borrow_repo,
            user_deposit_repo,
            borrower_repo,
            metric_repo,
            shared_object_repo,
        }
    }

    pub fn save_borrower_to_db(
        &self,
        borrower: crate::types::Borrower,
    ) -> Result<models::borrower::Borrower> {
        let borrower = match self
            .borrower_repo
            .find_by_platform_and_address(&borrower.platform, &borrower.borrower)
        {
            Ok(existing_borrower) => {
                let update_borrower = db::models::borrower::UpdateBorrower {
                    platform: None,
                    borrower: None,
                    obligation_id: borrower.obligation_id.clone(),
                    status: Some(borrower.status),
                };
                self.borrower_repo
                    .update(existing_borrower.id, &update_borrower)?
            }
            Err(_) => {
                let new_borrower = db::models::borrower::NewBorrower {
                    platform: borrower.platform.clone(),
                    borrower: borrower.borrower.clone(),
                    obligation_id: borrower.obligation_id.clone(),
                    status: borrower.status,
                };
                self.borrower_repo.create(&new_borrower)?
            }
        };

        Ok(borrower)
    }

    pub fn update_borrower_status_to_db(
        &self,
        platform: &str,
        borrower: &str,
        status: i32,
    ) -> Result<models::borrower::Borrower> {
        match self
            .borrower_repo
            .find_by_platform_and_address(platform, borrower)
        {
            Ok(existing_borrower) => {
                let update_borrower = UpdateBorrower {
                    platform: None,
                    borrower: None,
                    obligation_id: None,
                    status: Some(status),
                };
                let borrower_m = self
                    .borrower_repo
                    .update(existing_borrower.id, &update_borrower)?;

                info!(
                    "Borrower {} on platform {} updated successfully",
                    borrower, platform
                );
                Ok(borrower_m)
            }
            Err(_) => {
                error!(
                    "Borrower {} on platform {} not found for status update",
                    borrower, platform
                );
                Err(anyhow!(
                    "Borrower {} on platform {} not found for status update",
                    borrower,
                    platform
                ))
            }
        }
    }

    pub async fn delete_borrower_portfolio_from_db(
        &self,
        platform: &str,
        borrower: &str,
    ) -> Result<()> {
        self.user_borrow_repo
            .delete_by_platform_and_address(platform, borrower)?;
        self.user_deposit_repo
            .delete_by_platform_and_address(platform, borrower)?;

        Ok(())
    }

    pub async fn save_user_borrow_to_db(
        &self,
        user_borrow: crate::types::UserBorrow,
    ) -> Result<()> {
        let user_borrow = match self
            .user_borrow_repo
            .find_by_platform_and_address_and_coin_type(
                &user_borrow.platform,
                &user_borrow.borrower,
                &user_borrow.coin_type,
            ) {
            Ok(existing_borrow) => {
                let update_borrow = user_borrow::UpdateUserBorrow {
                    platform: None,
                    borrower: None,
                    coin_type: None,
                    amount: Some(user_borrow.amount),
                    obligation_id: user_borrow.obligation_id.clone(),
                    debt_borrow_index: user_borrow.debt_borrow_index.clone(),
                };

                self.user_borrow_repo
                    .update(existing_borrow.id, &update_borrow)?
            }
            Err(_) => {
                let new_borrow = user_borrow::NewUserBorrow {
                    platform: user_borrow.platform.clone(),
                    borrower: user_borrow.borrower.clone(),
                    coin_type: user_borrow.coin_type.clone(),
                    amount: user_borrow.amount.clone(),
                    obligation_id: user_borrow.obligation_id.clone(),
                    debt_borrow_index: user_borrow.debt_borrow_index.clone(),
                };

                self.user_borrow_repo.create(&new_borrow)?
            }
        };

        Ok(())
    }

    pub async fn save_user_deposit_to_db(
        &self,
        user_deposit: crate::types::UserDeposit,
    ) -> Result<()> {
        let user_deposit = match self
            .user_deposit_repo
            .find_by_platform_and_address_and_coin_type(
                &user_deposit.platform,
                &user_deposit.borrower,
                &user_deposit.coin_type,
            ) {
            Ok(existing_deposit) => {
                let update_deposit = user_deposit::UpdateUserDeposit {
                    platform: None,
                    borrower: None,
                    coin_type: None,
                    amount: Some(user_deposit.amount),
                    obligation_id: user_deposit.obligation_id.clone(),
                };
                self.user_deposit_repo
                    .update(existing_deposit.id, &update_deposit)?
            }
            Err(_) => {
                let new_deposit = user_deposit::NewUserDeposit {
                    platform: user_deposit.platform.clone(),
                    borrower: user_deposit.borrower.clone(),
                    coin_type: user_deposit.coin_type.clone(),
                    amount: user_deposit.amount.clone(),
                    obligation_id: user_deposit.obligation_id.clone(),
                };
                self.user_deposit_repo.create(&new_deposit)?
            }
        };

        Ok(())
    }

    /// Saves the Pyth price to the database.
    /// This function will:
    /// 1. Find the coins associated with the Pyth feed ID.
    /// 2. Update the price of each coin in parallel.
    /// 3. Save the updated coins to the MEV database if `use_mev_db` is true.
    /// 4. Return the updated coins as a vector of `models::coin::Coin`.
    ///
    pub async fn save_pyth_price(
        &self,
        pyth_price: crate::types::PythPrice,
        use_hermes: bool,
    ) -> Result<Vec<models::coin::Coin>> {
        let coin_models = self
            .coin_repo
            .find_by_pyth_feed_id(&pyth_price.feed_id)
            .map_err(|e| {
                error!("Error finding coin by Pyth feed ID: {:?}", e);
                anyhow!("Error finding coin by Pyth feed ID")
            })?;

        info!(
            "Found {} coins for Pyth feed ID: {}",
            coin_models.len(),
            pyth_price.feed_id
        );

        // update coin price in parallel
        let updated_coins = if use_hermes {
            // if price hermes, update the price directly to the model
            coin_models
                .iter()
                .map(|coin_model| {
                    let update_coin = UpdateCoin {
                        coin_type: None,
                        decimals: None,
                        name: None,
                        symbol: None,
                        price_pyth: None,
                        price_supra: None,
                        price_switchboard: None,
                        pyth_feed_id: None,
                        pyth_info_object_id: None,
                        pyth_latest_updated_at: None,
                        pyth_ema_price: None,
                        pyth_decimals: Some(pyth_price.decimals as i32),
                        navi_asset_id: None,
                        navi_oracle_id: None,
                        navi_feed_id: None,
                        hermes_price: Some(pyth_price.spot_price.clone()),
                        hermes_latest_updated_at: Some(utils::timestamp_to_naive_datetime(
                            pyth_price.latest_updated_timestamp,
                        )),
                        vaa: pyth_price.vaa.clone(),
                    };
                    self.coin_repo.update(coin_model.id, &update_coin)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            coin_models
                .par_iter()
                .map(|coin_model| {
                    let update_coin = UpdateCoin {
                        coin_type: None,
                        decimals: None,
                        name: None,
                        symbol: None,
                        price_pyth: Some(pyth_price.spot_price.clone()),
                        price_supra: None,
                        price_switchboard: None,
                        pyth_feed_id: None,
                        pyth_info_object_id: None,
                        pyth_latest_updated_at: Some(utils::timestamp_to_naive_datetime(
                            pyth_price.latest_updated_timestamp,
                        )),
                        pyth_ema_price: Some(pyth_price.ema_price.clone()),
                        pyth_decimals: Some(pyth_price.decimals as i32),
                        navi_asset_id: None,
                        navi_oracle_id: None,
                        navi_feed_id: None,
                        hermes_price: None,
                        hermes_latest_updated_at: None,
                        vaa: None,
                    };

                    self.coin_repo.update(coin_model.id, &update_coin)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(updated_coins)
    }

    pub fn save_metric_to_db(&self, metric: crate::types::Metric) -> Result<()> {
        let seq_number = metric.latest_seq_number;
        //let new_metric: db::models::metric::NewMetric = metric.into();
        let new_metric = db::models::metric::NewMetric::from(metric);

        // Save the metric to the database
        if let Err(e) = self.metric_repo.create(&new_metric) {
            error!(
                "Failed to save metrics for checkpoint #{}: {}",
                seq_number, e
            );
            return Err(anyhow!(
                "Error saving metrics for checkpoint #{}: {}",
                seq_number,
                e
            ));
        }

        warn!("Saved metrics for checkpoint #{}", seq_number);

        Ok(())
    }

    pub fn save_shared_object_to_db(
        &self,
        object_id: &str,
        initial_shared_version: u64,
    ) -> Result<models::shared_object::SharedObject> {
        let shared_object = match self.shared_object_repo.find_by_object_id(object_id) {
            Ok(existing_object) => {
                let update_object = db::models::shared_object::UpdateSharedObject {
                    object_id: None,
                    initial_shared_version: Some(initial_shared_version as i64),
                };
                self.shared_object_repo
                    .update(existing_object.id, &update_object)?
            }
            Err(_) => {
                let new_object = db::models::shared_object::NewSharedObject {
                    object_id: object_id.to_string(),
                    initial_shared_version: initial_shared_version as i64,
                };
                self.shared_object_repo.create(&new_object)?
            }
        };

        Ok(shared_object)
    }

    pub async fn find_user_borrows_with_coin_info(
        &self,
        platform: &str,
        borrower: &str,
        use_hermes: bool,
    ) -> Result<Vec<user_borrow::UserBorrowWithCoinInfo>> {
        self.user_borrow_repo
            .find_by_platform_and_address_with_coin_info(platform, borrower)
            .map_err(|e| {
                anyhow!(
                    "Failed to find user borrows with coin info for {} on platform {}: {}",
                    borrower,
                    platform,
                    e
                )
            })
    }

    pub async fn find_user_deposits_with_coin_info(
        &self,
        platform: &str,
        borrower: &str,
        use_hermes: bool,
    ) -> Result<Vec<user_deposit::UserDepositWithCoinInfo>> {
        self.user_deposit_repo
            .find_by_platform_and_address_with_coin_info(platform, borrower)
            .map_err(|e| {
                anyhow!(
                    "Failed to find user deposits with coin info for {} on platform {}: {}",
                    borrower,
                    platform,
                    e
                )
            })
    }

    /// Finds all borrower coins for a given borrower address.
    /// It gathers the borrower's assets from both user borrows and user deposits,
    /// ensuring that the debt coin is included if it is not already present.
    /// /// The assets are represented as tuples of (coin_type, asset_id, pyth_object_id).
    /// /// Returns a Result indicating success or failure.
    /// # Arguments
    /// * `platform` - The lending platform
    /// * `borrower` - The address of the borrower for whom to find coins.
    /// # Returns
    /// * `Result<HashSet(coin_type, asset_id, pyth_info_object_id, navi_feed_id)>` - Ok if successful, or an error if something goes wrong
    ///
    pub fn find_borrower_coins(
        &self,
        platform: &str,
        borrower: &str,
    ) -> Result<HashSet<BorrowerAsset>> {
        // gather borrower's assets in a set to avoid duplicates
        // each asset is represented as a tuple of (coin_type, asset_id, pyth_object_id)
        let mut assets = HashSet::new();

        let user_borrows = self
            .user_borrow_repo
            .find_by_platform_and_address_with_coin_info(platform, borrower)?;
        for user_borrow in user_borrows {
            let pyth_info_object_id = user_borrow
                .pyth_info_object_id
                .as_deref()
                .ok_or_else(|| {
                    anyhow!(
                        "Pyth info object ID not found for user borrow {} in market model",
                        user_borrow.coin_type
                    )
                })?
                .to_string();

            assets.insert(BorrowerAsset {
                coin_type: user_borrow.coin_type,
                asset_id: user_borrow.asset_id,
                pyth_info_object_id,
                navi_feed_id: user_borrow.navi_feed_id,
                vaa: user_borrow.vaa,
            });
        }

        let user_deposits = self
            .user_deposit_repo
            .find_by_platform_and_address_with_coin_info(platform, borrower)?;

        for user_deposit in user_deposits {
            let pyth_info_object_id = user_deposit
                .pyth_info_object_id
                .as_deref()
                .ok_or_else(|| {
                    anyhow!(
                        "Pyth info object ID not found for user deposit {} in market model",
                        user_deposit.coin_type
                    )
                })?
                .to_string();

            assets.insert(BorrowerAsset {
                coin_type: user_deposit.coin_type,
                asset_id: user_deposit.asset_id,
                pyth_info_object_id,
                navi_feed_id: user_deposit.navi_feed_id,
                vaa: user_deposit.vaa,
            });
        }

        Ok(assets)
    }

    pub fn find_obligation_id_given_borrower_and_debt_coin(
        &self,
        platform: &str,
        borrower: &str,
        debt_coin: &str,
    ) -> Result<Option<String>> {
        let user_borrow = self
            .user_borrow_repo
            .find_by_platform_and_address_and_coin_type(platform, borrower, debt_coin)?;

        Ok(user_borrow.obligation_id)
    }

    pub fn find_obligation_id_given_borrower(
        &self,
        platform: &str,
        borrower: &str,
    ) -> Result<Option<String>> {
        let user_borrow = self
            .user_borrow_repo
            .find_by_platform_and_address(platform, borrower)?;

        let user_deposit = self
            .user_deposit_repo
            .find_by_platform_and_address(platform, borrower)?;

        if !user_borrow.is_empty() {
            return Ok(user_borrow[0].obligation_id.clone());
        }

        if !user_deposit.is_empty() {
            return Ok(user_deposit[0].obligation_id.clone());
        }

        // Assuming the first borrow is the one we want
        Ok(None)
    }

    pub fn find_coin_by_type(&self, coin_type: &str) -> Result<Coin> {
        self.coin_repo.find_by_coin_type(coin_type).map_err(|e| {
            error!("Failed to find coin by type {}: {}", coin_type, e);
            anyhow!("Error finding coin by type: {}", e)
        })
    }

    pub fn find_borrower_given_obligation_id(
        &self,
        platform: &str,
        obligation_id: &str,
    ) -> Result<String> {
        let user_borrow = self
            .user_borrow_repo
            .find_by_platform_and_obligation_id(platform, obligation_id)?;

        Ok(user_borrow.borrower)
    }

    /// Finds all borrowers with a specific status.
    /// This method is used in one-off command to syncronized all borrowers portfolios to DB
    ///
    pub fn find_borrowers_by_status(&self, status: i32) -> Result<Vec<Borrower>> {
        self.borrower_repo
            .find_all_by_status(status)
            .map_err(|e| anyhow!("Error finding borrowers by status {}: {}", status, e))
    }

    pub fn find_latest_seq_number(&self) -> Result<Option<db::models::metric::Metric>> {
        self.metric_repo
            .find_latest_seq_number()
            .map_err(|e| anyhow!("Error finding latest seq number: {}", e))
    }

    pub fn find_all_pyth_feed_ids(&self) -> Result<Vec<String>> {
        self.coin_repo
            .find_all_pyth_feed_ids()
            .map_err(|e| anyhow!("Error finding all Pyth feed IDs: {}", e))
    }

    pub fn find_borrower_by_platform_and_address(
        &self,
        platform: &str,
        address: &str,
    ) -> Result<Borrower> {
        self.borrower_repo
            .find_by_platform_and_address(platform, address)
            .map_err(|e| {
                anyhow!(
                    "Error finding borrower by platform {} and address {}: {}",
                    platform,
                    address,
                    e
                )
            })
    }

    pub async fn find_all_borrowers_by_status(
        &self,
        status: i32,
    ) -> Result<Vec<models::borrower::Borrower>> {
        self.borrower_repo
            .find_all_by_status(status)
            .map_err(|e| anyhow!("Error finding all borrowers by status {}: {}", status, e))
    }

    pub fn find_distinct_user_borrows(
        &self,
    ) -> Result<Vec<models::user_borrow::UserBorrowDistinct>> {
        self.user_borrow_repo
            .find_distinct_platform_and_address()
            .map_err(|e| anyhow!("Error finding distinct user borrows: {}", e))
    }

    pub fn find_distinct_user_deposits(
        &self,
    ) -> Result<Vec<models::user_deposit::UserDepositDistinct>> {
        self.user_deposit_repo
            .find_distinct_platform_and_address()
            .map_err(|e| anyhow!("Error finding distinct user deposits: {}", e))
    }

    pub fn find_shared_object_by_id(
        &self,
        object_id: &str,
    ) -> Result<models::shared_object::SharedObject> {
        self.shared_object_repo
            .find_by_object_id(object_id)
            .map_err(|e| anyhow!("Error finding shared object by ID {}: {}", object_id, e))
    }
}
