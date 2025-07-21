use crate::{
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::db_service::pool::PoolService,
    service::dex::DEXService,
    utils,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{self, NewPool, Pool, UpdatePool},
};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
struct SwapEventV2 {
    pool_id: ObjectID,
    issuer: SuiAddress,
    referrer: Option<SuiAddress>,
    types_in: Vec<String>,
    amounts_in: Vec<u64>,
    types_out: Vec<String>,
    amounts_out: Vec<u64>,
    reserves: Vec<u64>,
}
pub struct Aftermath {
    exchange: String,
    sui_client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pool_service: Arc<PoolService>,
    dex_service: Arc<dyn DEXService + Send + Sync>,
}

impl Aftermath {
    pub fn new(
        sui_client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_service: Arc<PoolService>,
        dex_service: Arc<dyn DEXService + Send + Sync>,
    ) -> Self {
        Aftermath {
            exchange: constant::AFTERMATH_EXCHANGE.to_string(),
            sui_client,
            pool_repo,
            coin_repo,
            pool_service,
            dex_service,
        }
    }
}

impl Display for Aftermath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AftermathEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Aftermath {
    /// Process tx event retrieving by Read API
    /// this is mostly used for development purposes
    ///
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::AFTERMATH_SWAP_EVENT => {
                let pool_id = data
                    .get("pool_id")
                    .ok_or_else(|| anyhow!("Missing pool field in event data"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("Pool field is not a string in event data"))?;

                self.process_pool(pool_id).await?;

                Ok(())
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    /// Process raw event from checkpoint data
    ///
    async fn process_raw_event(
        &self,
        event_type: &str,
        sender: &str,
        event: Event,
        tx_digest: &str,
    ) -> Result<OnchainEvent> {
        match event_type {
            constant::AFTERMATH_SWAP_EVENT => {
                info!("Processing Onchain swap event: {:?}", event);
                let pool_id = self.extract_pool_id_from_event(&event)?;

                let pool = self.process_pool(&pool_id).await?;

                Ok(OnchainEvent::DEXSwap(indexer::DEXSwapEvent {
                    exchange: self.exchange.clone(),
                    pool_id: pool_id.clone(),
                }))
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    /// Generate a unique event ID to identify the event across checkpoint data.
    /// This ID helps processing a particular pool only once per checkpoint.
    ///
    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        let pool_id = self.extract_pool_id_from_event(event)?;

        Ok(format!("{}_{}_{}", &self.exchange, event_type, &pool_id))
    }
}

impl Aftermath {
    async fn process_pool(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let pool = self.dex_service.get_pool_data(pool_id).await?;

        // save to database
        self.pool_service.save_pool_to_db(pool.clone()).await?;

        Ok(pool)
    }

    fn extract_pool_id_from_event(&self, event: &Event) -> Result<String> {
        let event_type = event.type_.to_string();
        let pool_id = match event_type.as_str() {
            constant::AFTERMATH_SWAP_EVENT => {
                let data = bcs::from_bytes::<SwapEventV2>(&event.contents)?;
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
