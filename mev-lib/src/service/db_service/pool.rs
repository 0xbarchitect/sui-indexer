use crate::{
    config::{self, Config},
    constant, indexer,
    service::registry::ServiceRegistry,
    utils,
};
use db::models::{
    self,
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{NewPool, Pool, UpdatePool},
    pool_tick::{NewPoolTick, PoolTick, UpdatePoolTick},
};
use db::repositories::{CoinRepository, PoolRepository, PoolTickRepository};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use rust_decimal::{prelude::*, Decimal};
use std::sync::Arc;
use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, trace, warn};

pub struct PoolService {
    config: Arc<Config>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_tick_repo: Arc<dyn PoolTickRepository + Send + Sync>,
}

impl PoolService {
    pub fn new(
        config: Arc<Config>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_tick_repo: Arc<dyn PoolTickRepository + Send + Sync>,
    ) -> Self {
        PoolService {
            config,
            pool_repo,
            coin_repo,
            pool_tick_repo,
        }
    }

    /// Saves the pool data to the database.
    /// This function will:
    /// 1. Save the pool with associated coins to the MEV database.
    /// 2. Save the pool to the persistent database.
    /// 3. Save the coins associated with the pool to the persistent database.
    ///
    pub async fn save_pool_to_db(&self, pool: crate::types::Pool) -> Result<()> {
        // sync pool data to persistent DB
        let pool_coins = pool.coins.clone();
        let pool_id = pool.pool_id.clone();

        let coins = pool
            .coins
            .iter()
            .map(|c| c.coin_type.clone())
            .collect::<Vec<String>>()
            .join(",");

        let coin_amounts = pool.coin_amounts.as_ref().map(|amounts| {
            amounts
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(",")
        });

        let weights = pool.weights.as_ref().map(|weights| {
            weights
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(",")
        });

        let fees_swap_in = pool.fees_swap_in.as_ref().map(|fees| {
            fees.iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(",")
        });

        let fees_swap_out = pool.fees_swap_out.as_ref().map(|fees| {
            fees.iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(",")
        });

        match self.pool_repo.find_by_address(&pool_id) {
            Ok(pool_model) => {
                let update_pool = UpdatePool {
                    exchange: Some(pool.exchange.clone()),
                    address: Some(pool.pool_id.clone()),
                    coins: Some(coins),
                    coin_amounts,
                    weights,
                    liquidity: pool.liquidity.clone(),
                    current_sqrt_price: pool.current_sqrt_price.clone(),
                    current_tick_index: pool.current_tick_index,
                    tick_spacing: pool.tick_spacing,
                    fee_rate: pool.fee_rate,
                    is_pause: pool.is_pause,
                    fees_swap_in,
                    fees_swap_out,
                    pool_type: pool.pool_type.clone(),
                };

                let _ = self.pool_repo.update(pool_model.id, &update_pool)?;
                info!("Updated pool {} in DB", pool_id);
            }
            Err(e) => {
                let new_pool = NewPool {
                    exchange: pool.exchange.clone(),
                    address: pool.pool_id.clone(),
                    coins,
                    coin_amounts,
                    weights,
                    liquidity: pool.liquidity.clone(),
                    current_sqrt_price: pool.current_sqrt_price.clone(),
                    current_tick_index: pool.current_tick_index,
                    tick_spacing: pool.tick_spacing,
                    fee_rate: pool.fee_rate,
                    is_pause: pool.is_pause,
                    fees_swap_in,
                    fees_swap_out,
                    pool_type: pool.pool_type.clone(),
                };

                let _ = self.pool_repo.create(&new_pool)?;
                info!("Created new pool {} in DB", pool_id);
            }
        }

        for coin in pool_coins.iter() {
            if let Err(e) = self.save_coin_to_db(coin.clone()).await {
                return Err(anyhow!(
                    "Failed to save coin {} to DB: {}",
                    coin.coin_type,
                    e
                ));
            }
        }

        Ok(())
    }

    /// Saves the pool tick data to the database.
    /// This function will:
    /// 1. Save the pool tick to the MEV database.
    /// 2. Check if the pool tick exists in the persistent database.
    ///   - If it exists, update the existing pool tick.
    ///  - If it does not exist, create a new pool tick.
    ///
    pub async fn save_pool_tick_to_db(&self, pool_tick: &PoolTick) -> Result<()> {
        let pool_tick_model = self
            .pool_tick_repo
            .find_by_address_and_tick_index(&pool_tick.address, pool_tick.tick_index);

        match pool_tick_model {
            Ok(pool_tick_model) => {
                info!("PoolTick found: {:?}, update it", pool_tick_model);

                let update_pool_tick = UpdatePoolTick {
                    address: Some(pool_tick.address.clone()),
                    tick_index: Some(pool_tick.tick_index),
                    liquidity_net: pool_tick.liquidity_net.clone(),
                    liquidity_gross: pool_tick.liquidity_gross.clone(),
                };

                let updated_pool_tick = self
                    .pool_tick_repo
                    .update(pool_tick_model.id, &update_pool_tick)?;
                info!("Updated PoolTick: {:?}", updated_pool_tick);
            }
            Err(e) => {
                info!("PoolTick not found in DB {:?}, create new one", e);
                let new_pool_tick = NewPoolTick {
                    address: pool_tick.address.clone(),
                    tick_index: pool_tick.tick_index,
                    liquidity_net: pool_tick.liquidity_net.clone(),
                    liquidity_gross: pool_tick.liquidity_gross.clone(),
                };

                let created_pool_tick = self.pool_tick_repo.create(&new_pool_tick)?;
                info!("Created new PoolTick: {:?}", created_pool_tick);
            }
        }

        Ok(())
    }

    pub async fn save_coin_to_db(&self, coin: crate::types::Coin) -> Result<models::coin::Coin> {
        let coin_model = self.coin_repo.find_by_coin_type(&coin.coin_type);

        match coin_model {
            Ok(coin_model) => {
                let update_coin = UpdateCoin {
                    coin_type: Some(coin.coin_type.clone()),
                    decimals: None, // TODO: do not update decimals, they should not change
                    name: coin.name.clone(),
                    symbol: coin.symbol.clone(),
                    price_pyth: None,
                    price_supra: None,
                    price_switchboard: None,
                    pyth_feed_id: coin.pyth_feed_id.clone(),
                    pyth_info_object_id: coin.pyth_info_object_id.clone(),
                    pyth_latest_updated_at: None,
                    pyth_ema_price: None,
                    pyth_decimals: None,
                    navi_asset_id: None,
                    navi_oracle_id: None,
                    navi_feed_id: None,
                    hermes_price: None,
                    hermes_latest_updated_at: None,
                    vaa: None,
                };

                self.coin_repo
                    .update(coin_model.id, &update_coin)
                    .map_err(|e| anyhow!("Failed to update coin {}: {}", coin.coin_type, e))
            }
            Err(e) => {
                let new_coin = NewCoin {
                    coin_type: coin.coin_type.clone(),
                    decimals: coin.decimals as i32,
                    name: coin.name.clone(),
                    symbol: coin.symbol.clone(),
                    price_pyth: None,
                    price_supra: None,
                    price_switchboard: None,
                    pyth_feed_id: coin.pyth_feed_id.clone(),
                    pyth_info_object_id: coin.pyth_info_object_id.clone(),
                    pyth_latest_updated_at: None,
                    pyth_ema_price: None,
                    pyth_decimals: None,
                    navi_asset_id: None,
                    navi_oracle_id: None,
                    navi_feed_id: None,
                    hermes_price: None,
                    hermes_latest_updated_at: None,
                    vaa: None,
                };
                let created_coin = self.coin_repo.create(&new_coin)?;
                info!("Created new coin {} in DB", created_coin.coin_type);

                Ok(created_coin)
            }
        }
    }

    /// Retrieves pool and its coins from the database.
    /// If `use_mev_db` is true, it will lookup data from the MEV database.
    /// If `shio_auction_digest` is provided, it will be used to filter the pool data.
    /// By default, the data is looked up from the persistent DB.
    ///
    /// Return a tuple (pool, vec<coin>).
    /// - Pool : db::models::pool::Pool
    /// - Vec<Coin> : List of db::models::coin::Coin associated with the pool.
    ///
    pub async fn find_pool_from_db(
        &self,
        pool_id: &str,
        shio_auction_digest: Option<String>,
    ) -> Result<(db::models::pool::Pool, Vec<db::models::coin::Coin>)> {
        let pool = self
            .pool_repo
            .find_by_address(pool_id)
            .map_err(|e| anyhow!("Failed to find pool: {}", e))?;

        let coins = pool.coins.split(',').collect::<Vec<_>>();
        let coins_len = coins.len();

        let coin_models = coins
            .into_iter()
            .map(|coin_type| {
                self.coin_repo
                    .find_by_coin_type(coin_type)
                    .map_err(|e| anyhow!("Failed to find coin {}: {}", coin_type, e))
            })
            .collect::<Result<Vec<_>>>()?;

        if coin_models.len() != coins_len {
            return Err(anyhow!(
                "Not all coins found for pool {}: expected {}, found {}",
                pool.id,
                coins_len,
                coin_models.len()
            ));
        }

        Ok((pool, coin_models))
    }

    /// Retrieves a weighted pool with its associated coins.
    /// The coin data is in format of tuples:
    /// (coin_type, weight, amount, decimals, fee_rate).
    ///
    pub async fn find_weighted_pool_from_db(
        &self,
        pool_id: &str,
        coin_type_out: &str,
        coin_type_in: &str,
        shio_auction_digest: Option<String>,
    ) -> Result<(
        models::pool::Pool,
        (String, Decimal, Decimal, i32, Decimal),
        (String, Decimal, Decimal, i32, Decimal),
    )> {
        let (pool, coin_models) = self.find_pool_from_db(pool_id, shio_auction_digest).await?;

        let coins = pool
            .coins
            .split(',')
            .collect::<Vec<_>>()
            .into_iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>();

        if coins.len() < 2 {
            return Err(anyhow!("Pool must have at least two coins"));
        }

        if !coins.contains(&coin_type_out.to_string()) || !coins.contains(&coin_type_in.to_string())
        {
            return Err(anyhow!(
                "Coin type {},{} is not part of the pool {}",
                coin_type_out,
                coin_type_in,
                pool.id
            ));
        }

        let weights = pool
            .weights
            .as_deref()
            .ok_or_else(|| anyhow!("Pool {} does not have weights", pool.id))?
            .split(',')
            .collect::<Vec<_>>()
            .iter()
            .map(|s| Decimal::from_str(s).map_err(|e| anyhow!("Failed to parse weight: {}", e)))
            .collect::<Result<Vec<_>, _>>()?;

        if weights.len() != coins.len() {
            return Err(anyhow!(
                "Weights length {} does not match coins length {} in pool {}",
                weights.len(),
                coins.len(),
                pool.id
            ));
        }

        let coin_amounts = pool
            .coin_amounts
            .as_deref()
            .ok_or_else(|| anyhow!("Pool {} does not have coin amounts", pool.id))?
            .split(',')
            .collect::<Vec<_>>()
            .into_iter()
            .map(|s| {
                Decimal::from_str(s).map_err(|e| anyhow!("Failed to parse coin amount: {}", e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if coin_amounts.len() != coins.len() {
            return Err(anyhow!(
                "Coin amounts length {} does not match coins length {} in pool {}",
                coin_amounts.len(),
                coins.len(),
                pool.id
            ));
        }

        let fees_swap_in = pool
            .fees_swap_in
            .as_deref()
            .ok_or_else(|| anyhow!("Pool {} does not have fees_swap_in", pool.id))?
            .split(',')
            .collect::<Vec<_>>()
            .into_iter()
            .map(|s| {
                Decimal::from_str(s).map_err(|e| anyhow!("Failed to parse fees_swap_in: {}", e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if fees_swap_in.len() != coins.len() {
            return Err(anyhow!(
                "Fees swap in length {} does not match coins length {} in pool {}",
                fees_swap_in.len(),
                coins.len(),
                pool.id
            ));
        }

        let coin_decimals = coin_models.iter().map(|c| c.decimals).collect::<Vec<_>>();
        if coin_decimals.len() != coins.len() {
            return Err(anyhow!(
                "Coin decimals length {} does not match coins length {} in pool {}",
                coin_decimals.len(),
                coins.len(),
                pool.id
            ));
        }

        // Associate coin type with its weight, amount, decimals
        // returns vector of tuples (coin_type, weight, amount, decimals, fee_swap_in)
        let coins = coins
            .into_iter()
            .zip(weights)
            .zip(coin_amounts)
            .zip(coin_decimals)
            .zip(fees_swap_in)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(((c, a), d), f)| (c.0, c.1, a, d, f))
            .collect::<Vec<_>>();

        let coin_out = coins
            .iter()
            .find(|(c, _, _, _, _)| c == coin_type_out)
            .ok_or_else(|| anyhow!("Coin type {} not found in pool {}", coin_type_out, pool.id))?;

        let coin_in = coins
            .iter()
            .find(|(c, _, _, _, _)| c == coin_type_in)
            .ok_or_else(|| anyhow!("Coin type {} not found in pool {}", coin_type_in, pool.id))?;

        Ok((pool, coin_in.clone(), coin_out.clone()))
    }

    /// Retrieves the next initialized tick for a given pool and tick index.
    /// If `zero_to_one` is true, the price goes down, so it will find the next lower tick.
    /// If `zero_to_one` is false, the price goes up, so it will find the next higher tick.
    /// If `use_mev_db` is true, it will lookup data from the MEV database.
    /// Otherwise, it will lookup data from the persistent DB.
    ///
    /// Return an `Option<PoolTick>`, which is the next initialized tick.
    /// If no tick is found, it returns `None`.
    ///
    pub async fn find_next_initialized_tick(
        &self,
        pool_id: &str,
        tick_index: i32,
        zero_to_one: bool,
    ) -> Result<Option<PoolTick>> {
        if zero_to_one {
            // swap token0 for token1, thus price goes down, so we need the next lower tick
            let next_tick = self
                .pool_tick_repo
                .find_lower_tick_for_address(pool_id, tick_index)?;

            Ok(next_tick)
        } else {
            // swap token1 for token0, thus price goes up, so we need the next higher tick
            let next_tick = self
                .pool_tick_repo
                .find_higher_tick_for_address(pool_id, tick_index)?;

            Ok(next_tick)
        }
    }

    pub async fn find_coin_by_type(&self, coin_type: &str) -> Result<models::coin::Coin> {
        self.coin_repo
            .find_by_coin_type(coin_type)
            .map_err(|e| anyhow!("Failed to find coin {}: {}", coin_type, e))
    }
}
