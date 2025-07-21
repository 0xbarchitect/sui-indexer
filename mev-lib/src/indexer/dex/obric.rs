use crate::{
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{db_service::pool::PoolService, dex::DEXService},
    utils,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{NewPool, Pool, UpdatePool},
};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::{base_types::ObjectID, event::Event};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct TypeName {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ObricSwapEvent {
    pool_id: ObjectID,
    amount_in: u64,
    amount_out: u64,
    a2b: bool,
    by_amount_in: bool,
    coin_a: TypeName,
    coin_b: TypeName,
}

pub struct Obric {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl Obric {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        Obric {
            exchange: constant::OBRIC_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for Obric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObricEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Obric {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::OBRIC_SWAP_EVENT => {
                let pool_id = data
                    .get("pool_id")
                    .ok_or(anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or(anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;

                Ok(())
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
            constant::OBRIC_SWAP_EVENT => {
                let pool_id = self.extract_pool_id_from_event(&event)?;
                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id,
                }))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        let pool_id = self.extract_pool_id_from_event(event)?;
        let event_type = event.type_.to_string();

        Ok(format!("{}_{}_{}", &self.exchange, &event_type, &pool_id))
    }
}

impl Obric {
    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await?;

        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    fn extract_pool_id_from_event(&self, event: &Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::OBRIC_SWAP_EVENT => {
                let data = bcs::from_bytes::<ObricSwapEvent>(&event.contents)?;
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
