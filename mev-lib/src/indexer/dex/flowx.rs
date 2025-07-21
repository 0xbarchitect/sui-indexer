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
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectID, SuiAddress};
use sui_types::event::{self, Event};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct Swap {
    sender: SuiAddress,
    pool_id: ObjectID,
    x_for_y: bool,
    amount_x: u64,
    amount_y: u64,
    sqrt_price_before: u128,
    sqrt_price_after: u128,
    liquidity: u128,
    tick_index: I32,
    fee_amount: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct ModifyLiquidityJson {
    sender: SuiAddress,
    pool_id: ObjectID,
    position_id: ObjectID,
    tick_lower_index: I32,
    tick_upper_index: I32,
    liquidity_delta: I128Json,
    #[serde_as(as = "DisplayFromStr")]
    amount_x: u64,
    #[serde_as(as = "DisplayFromStr")]
    amount_y: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModifyLiquidity {
    sender: SuiAddress,
    pool_id: ObjectID,
    position_id: ObjectID,
    tick_lower_index: I32,
    tick_upper_index: I32,
    liquidity_delta: I128,
    amount_x: u64,
    amount_y: u64,
}

pub struct FlowX {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl FlowX {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        FlowX {
            exchange: constant::FLOWX_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for FlowX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FlowXEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for FlowX {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::FLOWX_SWAP_EVENT => {
                let pool_id = data
                    .get("pool_id")
                    .ok_or(anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or(anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;
                Ok(())
            }
            constant::FLOWX_MODIFY_LIQUIDITY_EVENT => {
                let event: ModifyLiquidityJson = serde_json::from_value(data.clone())?;
                let event_raw = ModifyLiquidity {
                    sender: event.sender,
                    pool_id: event.pool_id,
                    position_id: event.position_id,
                    tick_lower_index: event.tick_lower_index,
                    tick_upper_index: event.tick_upper_index,
                    liquidity_delta: I128::from_json(&event.liquidity_delta),
                    amount_x: event.amount_x,
                    amount_y: event.amount_y,
                };

                self.process_modify_liquidity(&event_raw).await
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
            constant::FLOWX_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(&event)?;
                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id,
                }))
            }
            constant::FLOWX_MODIFY_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<ModifyLiquidity>(&event.contents)?;

                self.process_modify_liquidity(&data).await?;

                Ok(OnchainEvent::DEXLiquidity(indexer::DEXLiquidityEvent {
                    exchange: self.exchange.clone(),
                    pool_id: data.pool_id.to_string(),
                }))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    /// Returns a unique event ID based on the exchange name, event type, and pool ID.
    /// This ID can be used to identify events uniquely across txs in the checkpoint.
    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        match event_type {
            constant::FLOWX_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(event)?;

                Ok(format!("{}_{}_{}", &self.exchange, event_type, &pool_id))
            }
            constant::FLOWX_MODIFY_LIQUIDITY_EVENT => {
                let data = bcs::from_bytes::<ModifyLiquidity>(&event.contents)?;

                Ok(format!(
                    "{}_{}_{}_{}",
                    &self.exchange, event_type, data.pool_id, data.position_id,
                ))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }
}

impl FlowX {
    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await?;

        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    async fn process_modify_liquidity(&self, event: &ModifyLiquidity) -> Result<()> {
        let ticks = vec![event.tick_lower_index.bits, event.tick_upper_index.bits];

        for tick in ticks {
            let pool_tick = PoolTick {
                id: 0,
                address: event.pool_id.to_string(),
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

    fn extract_pool_id_from_event(&self, event: &sui_types::event::Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::FLOWX_SWAP_EVENT => {
                let data = bcs::from_bytes::<Swap>(&event.contents)?;
                info!("Swap event data: {:?}", data);
                data.pool_id.to_string()
            }
            _ => {
                return Err(anyhow!("Unknown event type: {}", event_type));
            }
        };

        Ok(pool_id)
    }
}
