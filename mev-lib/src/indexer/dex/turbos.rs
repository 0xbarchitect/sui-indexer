use crate::{
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{db_service::pool::PoolService, dex::DEXService},
    types::{I128Json, I128, I32},
    utils::tick_math,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{NewPool, Pool, UpdatePool},
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
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    event::Event,
};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct SwapEvent {
    pool: ObjectID,
    recipient: SuiAddress,
    amount_a: u64,
    amount_b: u64,
    liquidity: u128,
    tick_current_index: I32,
    tick_pre_index: I32,
    sqrt_price: u128,
    protocol_fee: u64,
    fee_amount: u64,
    a_to_b: bool,
    is_exact_in: bool,
}

// aka AddLiquidity event
#[derive(Debug, Deserialize, Serialize)]
struct MintEvent {
    pool: ObjectID,
    owner: SuiAddress,
    tick_lower_index: I32,
    tick_upper_index: I32,
    amount_a: u64,
    amount_b: u64,
    liquidity_delta: u128,
}

// aka RemoveLiquidity event
#[derive(Debug, Deserialize, Serialize)]
struct BurnEvent {
    pool: ObjectID,
    owner: SuiAddress,
    tick_lower_index: I32,
    tick_upper_index: I32,
    amount_a: u64,
    amount_b: u64,
    liquidity_delta: u128,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct MintEventJson {
    pool: ObjectID,
    owner: SuiAddress,
    tick_lower_index: I32,
    tick_upper_index: I32,
    #[serde_as(as = "DisplayFromStr")]
    amount_a: u64,
    #[serde_as(as = "DisplayFromStr")]
    amount_b: u64,
    #[serde_as(as = "DisplayFromStr")]
    liquidity_delta: u128,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct BurnEventJson {
    pool: ObjectID,
    owner: SuiAddress,
    tick_lower_index: I32,
    tick_upper_index: I32,
    #[serde_as(as = "DisplayFromStr")]
    amount_a: u64,
    #[serde_as(as = "DisplayFromStr")]
    amount_b: u64,
    #[serde_as(as = "DisplayFromStr")]
    liquidity_delta: u128,
}

pub struct Turbos {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl Turbos {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        Turbos {
            exchange: constant::TURBOS_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for Turbos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TurbosEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Turbos {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::TURBOS_SWAP_EVENT => {
                let pool_id = data
                    .get("pool")
                    .ok_or(anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or(anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;
                Ok(())
            }
            constant::TURBOS_ADD_LIQUIDITY_EVENT => {
                let event: MintEventJson = serde_json::from_value(data.clone())?;
                let event_raw = MintEvent {
                    pool: event.pool,
                    owner: event.owner,
                    tick_lower_index: event.tick_lower_index,
                    tick_upper_index: event.tick_upper_index,
                    amount_a: event.amount_a,
                    amount_b: event.amount_b,
                    liquidity_delta: event.liquidity_delta,
                };

                self.process_add_liquidity_event(&event_raw).await
            }
            constant::TURBOS_REMOVE_LIQUIDITY_EVENT => {
                let event: BurnEventJson = serde_json::from_value(data.clone())?;
                let event_raw = BurnEvent {
                    pool: event.pool,
                    owner: event.owner,
                    tick_lower_index: event.tick_lower_index,
                    tick_upper_index: event.tick_upper_index,
                    amount_a: event.amount_a,
                    amount_b: event.amount_b,
                    liquidity_delta: event.liquidity_delta,
                };

                self.process_remove_liquidity_event(&event_raw).await
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
            constant::TURBOS_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(&event)?;

                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id,
                }))
            }
            constant::TURBOS_ADD_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<MintEvent>(&event.contents)?;

                self.process_add_liquidity_event(&data).await?;

                Ok(OnchainEvent::DEXLiquidity(indexer::DEXLiquidityEvent {
                    exchange: self.exchange.clone(),
                    pool_id: data.pool.to_string(),
                }))
            }
            constant::TURBOS_REMOVE_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<BurnEvent>(&event.contents)?;

                self.process_remove_liquidity_event(&data).await?;

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
            constant::TURBOS_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(event)?;

                Ok(format!("{}_{}_{}", &self.exchange, event_type, &pool_id))
            }
            constant::TURBOS_ADD_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<MintEvent>(&event.contents)?;

                Ok(format!(
                    "{}_{}_{}_{}",
                    &self.exchange, event_type, data.pool, data.owner,
                ))
            }
            constant::TURBOS_REMOVE_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<BurnEvent>(&event.contents)?;

                Ok(format!(
                    "{}_{}_{}_{}",
                    &self.exchange, event_type, data.pool, data.owner,
                ))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }
}

impl Turbos {
    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await?;

        // save to database
        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    async fn process_add_liquidity_event(&self, event: &MintEvent) -> Result<()> {
        let ticks = vec![event.tick_lower_index.bits, event.tick_upper_index.bits];

        for tick in ticks {
            let pool_tick = PoolTick {
                id: 0, // ID will be auto-generated by the database
                address: event.pool.to_string(),
                tick_index: tick_math::i32_from_u32(tick)?,
                liquidity_gross: None,
                liquidity_net: None,
                created_at: None,
                updated_at: None,
            };

            self.pool_service.save_pool_tick_to_db(&pool_tick).await?;
        }

        Ok(())
    }

    async fn process_remove_liquidity_event(&self, event: &BurnEvent) -> Result<()> {
        let ticks = vec![event.tick_lower_index.bits, event.tick_upper_index.bits];

        for tick in ticks {
            let pool_tick = PoolTick {
                id: 0, // ID will be auto-generated by the database
                address: event.pool.to_string(),
                tick_index: tick_math::i32_from_u32(tick)?,
                liquidity_gross: None,
                liquidity_net: None,
                created_at: None, // Created at will be set by the database
                updated_at: None, // Updated at will be set by the database
            };

            self.pool_service.save_pool_tick_to_db(&pool_tick).await?;
        }

        Ok(())
    }

    fn extract_pool_id_from_event(&self, event: &sui_types::event::Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::TURBOS_SWAP_EVENT => {
                let data = bcs::from_bytes::<SwapEvent>(&event.contents)?;
                info!("Swap event data: {:?}", data);
                data.pool.to_string()
            }
            _ => {
                return Err(anyhow!("Unknown event type: {}", event_type));
            }
        };

        Ok(pool_id)
    }
}
