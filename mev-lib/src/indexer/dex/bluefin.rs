use crate::{
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{db_service::pool::PoolService, dex::DEXService},
    types::{I128Json, I128, I32},
    utils::tick_math,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{self, NewPool, Pool, UpdatePool},
    pool_tick::{NewPoolTick, PoolTick, UpdatePoolTick},
};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::object::{MoveObject, Object};
use sui_types::{base_types::ObjectID, event::Event};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct AssetSwap {
    pool_id: ObjectID,
    a2b: bool,
    amount_in: u64,
    amount_out: u64,
    pool_coin_a_amount: u64,
    pool_coin_b_amount: u64,
    fee: u64,
    before_liquidity: u128,
    after_liquidity: u128,
    before_sqrt_price: u128,
    after_sqrt_price: u128,
    current_tick: I32,
    exceeded: bool,
    sequence_number: u128,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct PoolTickUpdateJson {
    pool: ObjectID,
    index: I32,
    #[serde_as(as = "DisplayFromStr")]
    liquidity_gross: u128,
    liquidity_net: I128Json,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolTickUpdate {
    pool: ObjectID,
    index: I32,
    liquidity_gross: u128,
    liquidity_net: I128,
}

pub struct Bluefin {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl Bluefin {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        Bluefin {
            exchange: constant::BLUEFIN_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for Bluefin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BluefinEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Bluefin {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::BLUEFIN_SWAP_EVENT => {
                let pool_id = data
                    .get("pool_id")
                    .ok_or(anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or(anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;
                Ok(())
            }
            constant::BLUEFIN_TICK_UPDATED_EVENT => {
                let event: PoolTickUpdateJson = serde_json::from_value(data.clone())?;

                let event_raw = PoolTickUpdate {
                    pool: event.pool,
                    index: event.index,
                    liquidity_gross: event.liquidity_gross,
                    liquidity_net: I128::from_json(&event.liquidity_net),
                };

                self.process_tick_updated(&event_raw).await
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    async fn process_raw_event(
        &self,
        event_type: &str,
        sender: &str,
        event: Event,
        tx_digest: &str,
    ) -> Result<OnchainEvent> {
        match event_type {
            constant::BLUEFIN_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(&event)?;
                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id,
                }))
            }
            constant::BLUEFIN_TICK_UPDATED_EVENT => {
                info!("Processing raw event: {:?}", event);

                let data = bcs::from_bytes::<PoolTickUpdate>(&event.contents)?;
                info!("Parsed PoolTickUpdate: {:?}", data);

                self.process_tick_updated(&data).await?;

                Ok(OnchainEvent::DEXLiquidity(indexer::DEXLiquidityEvent {
                    exchange: self.exchange.clone(),
                    pool_id: data.pool.to_string(),
                }))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    /// Returns a unique event ID based on the exchange name, event type, and pool ID.
    /// This ID can be used to identify events uniquely across txs in the checkpoint.
    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        match event_type {
            constant::BLUEFIN_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(event)?;

                Ok(format!("{}_{}_{}", &self.exchange, &event_type, &pool_id))
            }
            constant::BLUEFIN_TICK_UPDATED_EVENT => {
                let data = bcs::from_bytes::<PoolTickUpdate>(&event.contents)?;

                Ok(format!(
                    "{}_{}_{}_{}",
                    &self.exchange,
                    &event_type,
                    &data.pool.to_string(),
                    &data.index.bits.to_string()
                ))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }
}

impl Bluefin {
    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await?;

        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    async fn process_tick_updated(&self, event: &PoolTickUpdate) -> Result<()> {
        info!("Processing pool-tick-update event: {:?}", event);

        let pool_tick = PoolTick {
            id: 0, // ID will be auto-generated by the database
            address: event.pool.to_string(),
            tick_index: tick_math::i32_from_u32(event.index.bits)?,
            liquidity_net: Some(event.liquidity_net.bits.to_string()),
            liquidity_gross: Some(event.liquidity_gross.to_string()),
            created_at: None, // Created at will be set by the database
            updated_at: None, // Updated at will be set by the database
        };

        self.pool_service.save_pool_tick_to_db(&pool_tick).await
    }

    fn extract_pool_id_from_event(&self, event: &Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::BLUEFIN_SWAP_EVENT => {
                let data = bcs::from_bytes::<AssetSwap>(&event.contents)?;
                info!("Swap event data: {:?}", data);
                data.pool_id.to_string()
            }
            constant::BLUEFIN_TICK_UPDATED_EVENT => {
                let data = bcs::from_bytes::<PoolTickUpdate>(&event.contents)?;
                info!("Tick update event data: {:?}", data);
                data.pool.to_string()
            }
            _ => {
                return Err(anyhow!("Unknown event type: {}", event_type));
            }
        };

        Ok(pool_id)
    }
}
