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
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::object::{MoveObject, Object};
use sui_types::{base_types::ObjectID, event::Event};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct SwapEvent {
    atob: bool,              // boolean (1 byte)
    pool: ObjectID,          // 32 bytes (ID)
    partner: ObjectID,       // 32 bytes (ID)
    amount_in: u64,          // 8 bytes
    amount_out: u64,         // 8 bytes
    ref_amount: u64,         // 8 bytes
    fee_amount: u64,         // 8 bytes
    vault_a_amount: u64,     // 8 bytes
    vault_b_amount: u64,     // 8 bytes
    before_sqrt_price: u128, // 16 bytes
    after_sqrt_price: u128,  // 16 bytes
    steps: u64,              // 8 bytes
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct AddLiquidityEventJson {
    pool: ObjectID,
    position: ObjectID,
    tick_lower: I32,
    tick_upper: I32,
    #[serde_as(as = "DisplayFromStr")]
    liquidity: u128,
    #[serde_as(as = "DisplayFromStr")]
    after_liquidity: u128,
    #[serde_as(as = "DisplayFromStr")]
    amount_a: u64,
    #[serde_as(as = "DisplayFromStr")]
    amount_b: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct RemoveLiquidityEventJson {
    pool: ObjectID,
    position: ObjectID,
    tick_lower: I32,
    tick_upper: I32,
    #[serde_as(as = "DisplayFromStr")]
    liquidity: u128,
    #[serde_as(as = "DisplayFromStr")]
    after_liquidity: u128,
    #[serde_as(as = "DisplayFromStr")]
    amount_a: u64,
    #[serde_as(as = "DisplayFromStr")]
    amount_b: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct AddLiquidityEvent {
    pool: ObjectID,
    position: ObjectID,
    tick_lower: I32,
    tick_upper: I32,
    liquidity: u128,
    after_liquidity: u128,
    amount_a: u64,
    amount_b: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct RemoveLiquidityEvent {
    pool: ObjectID,
    position: ObjectID,
    tick_lower: I32,
    tick_upper: I32,
    liquidity: u128,
    after_liquidity: u128,
    amount_a: u64,
    amount_b: u64,
}

pub struct Cetus {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl Cetus {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        Cetus {
            exchange: constant::CETUS_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for Cetus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CetusEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Cetus {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::CETUS_SWAP_EVENT => {
                info!("Processing swap event: {}", data);
                let pool_id = data
                    .get("pool")
                    .ok_or(anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or(anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;
                Ok(())
            }
            constant::CETUS_ADD_LIQUIDITY_EVENT => {
                info!("Processing add liquidity event: {}", data);
                let event: AddLiquidityEventJson = serde_json::from_value(data.clone())?;

                let event_raw = AddLiquidityEvent {
                    pool: event.pool,
                    position: event.position,
                    tick_lower: event.tick_lower,
                    tick_upper: event.tick_upper,
                    liquidity: event.liquidity,
                    after_liquidity: event.after_liquidity,
                    amount_a: event.amount_a,
                    amount_b: event.amount_b,
                };

                self.process_add_liquidity_event(&event_raw).await
            }
            constant::CETUS_REMOVE_LIQUIDITY_EVENT => {
                info!("Processing remove liquidity event: {}", data);
                let event: RemoveLiquidityEventJson = serde_json::from_value(data.clone())?;

                let event_raw = RemoveLiquidityEvent {
                    pool: event.pool,
                    position: event.position,
                    tick_lower: event.tick_lower,
                    tick_upper: event.tick_upper,
                    liquidity: event.liquidity,
                    after_liquidity: event.after_liquidity,
                    amount_a: event.amount_a,
                    amount_b: event.amount_b,
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
            constant::CETUS_SWAP_EVENT => {
                info!("Processing raw swap event: {:?}", event);
                let pool_id = self.extract_pool_id_from_event(&event)?;
                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id,
                }))
            }
            constant::CETUS_ADD_LIQUIDITY_EVENT => {
                info!("Processing add liquidity event: {:?}", event);
                let data = bcs::from_bytes::<AddLiquidityEvent>(&event.contents)?;
                self.process_add_liquidity_event(&data).await?;

                Ok(OnchainEvent::DEXLiquidity(indexer::DEXLiquidityEvent {
                    exchange: self.exchange.clone(),
                    pool_id: data.pool.to_string(),
                }))
            }
            constant::CETUS_REMOVE_LIQUIDITY_EVENT => {
                info!("Processing remove liquidity event: {:?}", event);
                let data = bcs::from_bytes::<RemoveLiquidityEvent>(&event.contents)?;
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
            constant::CETUS_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(event)?;

                Ok(format!("{}_{}_{}", &self.exchange, event_type, pool_id))
            }

            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }
}

impl Cetus {
    fn extract_pool_id_from_event(&self, event: &Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::CETUS_SWAP_EVENT => {
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

    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await.map_err(|e| {
            error!("Failed to get pool data: {}", e);
            e
        })?;

        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    async fn process_add_liquidity_event(&self, event: &AddLiquidityEvent) -> Result<()> {
        let ticks = vec![event.tick_lower.bits, event.tick_upper.bits];

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

    async fn process_remove_liquidity_event(&self, event: &RemoveLiquidityEvent) -> Result<()> {
        let ticks = vec![event.tick_lower.bits, event.tick_upper.bits];

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
}
