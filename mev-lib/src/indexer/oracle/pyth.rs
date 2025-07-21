use crate::{
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::db_service::{lending::LendingService, pool::PoolService},
    utils,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{NewPool, Pool, UpdatePool},
};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_types::event::Event;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, event, info, instrument, trace, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesPriceFeed {
    pub id: String,
    pub price: HermesPrice,
    pub ema_price: HermesPrice,
    pub vaa: String,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesPrice {
    #[serde_as(as = "DisplayFromStr")]
    pub price: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub conf: u64,
    pub expo: i32,
    pub publish_time: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct PriceFeedUpdateEventJson {
    pub price_feed: PriceFeedJson,
    #[serde_as(as = "DisplayFromStr")]
    pub timestamp: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct PriceFeedJson {
    pub price_identifier: PriceIdentifier,
    pub price: PriceJson,
    pub ema_price: PriceJson,
}

#[derive(Debug, Deserialize, Serialize)]
struct PriceIdentifier {
    bytes: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct I64Json {
    pub negative: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub magnitude: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct PriceJson {
    pub price: I64Json,
    #[serde_as(as = "DisplayFromStr")]
    pub conf: u64,
    pub expo: I64Json,
    #[serde_as(as = "DisplayFromStr")]
    pub timestamp: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PriceFeedUpdateEvent {
    pub price_feed: PriceFeed,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PriceFeed {
    pub price_identifier: PriceIdentifier,
    pub price: Price,
    pub ema_price: Price,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I64 {
    pub negative: bool,
    pub magnitude: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Price {
    pub price: I64,
    pub conf: u64,
    pub expo: I64,
    pub timestamp: u64,
}

pub struct Pyth {
    oracle_name: String,
    sui_client: Arc<SuiClient>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    db_pool_service: Arc<PoolService>,
    db_lending_service: Arc<LendingService>,
}

impl Pyth {
    pub fn new(
        sui_client: Arc<SuiClient>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        db_pool_service: Arc<PoolService>,
        db_lending_service: Arc<LendingService>,
    ) -> Self {
        Pyth {
            oracle_name: "pyth".to_string(),
            sui_client,
            coin_repo,
            db_pool_service,
            db_lending_service,
        }
    }
}

impl Display for Pyth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PythEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Pyth {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::PYTH_UPDATE_PRICE_EVENT => {
                let event: PriceFeedUpdateEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize event data: {:?}", e))?;

                info!("Pyth price update event: {:?}", event);

                let raw_event = PriceFeedUpdateEvent {
                    price_feed: PriceFeed {
                        price_identifier: PriceIdentifier {
                            bytes: event.price_feed.price_identifier.bytes,
                        },
                        price: Price {
                            price: I64 {
                                negative: event.price_feed.price.price.negative,
                                magnitude: event.price_feed.price.price.magnitude,
                            },
                            conf: event.price_feed.price.conf,
                            expo: I64 {
                                negative: event.price_feed.price.expo.negative,
                                magnitude: event.price_feed.price.expo.magnitude,
                            },
                            timestamp: event.timestamp,
                        },
                        ema_price: Price {
                            price: I64 {
                                negative: event.price_feed.ema_price.price.negative,
                                magnitude: event.price_feed.ema_price.price.magnitude,
                            },
                            conf: event.price_feed.ema_price.conf,
                            expo: I64 {
                                negative: event.price_feed.ema_price.expo.negative,
                                magnitude: event.price_feed.ema_price.expo.magnitude,
                            },
                            timestamp: event.timestamp,
                        },
                    },
                    timestamp: event.timestamp,
                };

                let onchain_event = self
                    .process_update_price_feed(event_type, raw_event)
                    .await?;
            }
            _ => {
                return Err(anyhow!("Unknown event type: {}", event_type));
            }
        }

        Ok(())
    }

    async fn process_raw_event(
        &self,
        event_type: &str,
        sender: &str,
        event: Event,
        tx_digest: &str,
    ) -> Result<OnchainEvent> {
        match event_type {
            constant::PYTH_UPDATE_PRICE_EVENT => {
                let event: PriceFeedUpdateEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode event: {:?}", e))?;

                info!("Pyth price update event: {:?}", event);

                self.process_update_price_feed(event_type, event).await
            }
            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }

    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        match event_type {
            constant::PYTH_UPDATE_PRICE_EVENT => {
                let event_data: PriceFeedUpdateEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode event: {:?}", e))?;

                let feed_id = utils::convert_number_vec_to_hex_string(
                    &event_data.price_feed.price_identifier.bytes,
                );

                // The feed ID is used as the event ID
                // In a checkpoint processing scenario, we will select the latest price update event
                // for each asset, ignoring all the previous events.
                Ok(format!("{}_{}_{}", &self.oracle_name, event_type, &feed_id))
            }
            _ => Err(anyhow!("Unknown Pyth event type: {}", event_type)),
        }
    }
}

impl Pyth {
    /// Processes the Pyth price feed update event.
    /// Extracts the price feed ID, EMA price, spot price, and decimals from the event data.
    /// Updates the corresponding coin models in the database.
    ///
    /// Returns a tuple (pyth_feed_id, spot_price, ema_price).
    /// - pyth_feed_id: String - The Pyth feed ID.
    /// - spot_price: String - The spot price of the coin.
    /// - ema_price: String - The EMA price of the coin.
    async fn process_update_price_feed(
        &self,
        event_type: &str,
        event_data: PriceFeedUpdateEvent,
    ) -> Result<OnchainEvent> {
        let feed_id =
            utils::convert_number_vec_to_hex_string(&event_data.price_feed.price_identifier.bytes);

        info!("Price feed ID: {:?}", feed_id);

        let pyth_price = crate::types::PythPrice {
            feed_id: feed_id.clone(),
            spot_price: event_data.price_feed.price.price.magnitude.to_string(),
            ema_price: event_data.price_feed.ema_price.price.magnitude.to_string(),
            decimals: event_data.price_feed.price.expo.magnitude as u8,
            latest_updated_timestamp: event_data.price_feed.price.timestamp,
            vaa: None,
        };

        // save to db
        self.db_lending_service
            .save_pyth_price(pyth_price, false)
            .await?;

        Ok(OnchainEvent::OraclePrice(indexer::OraclePriceEvent {
            oracle: self.oracle_name.clone(),
            feed_id: feed_id.clone(),
            spot_price: event_data.price_feed.price.price.magnitude.to_string(),
            ema_price: event_data.price_feed.ema_price.price.magnitude.to_string(),
            publish_time: event_data.price_feed.price.timestamp,
            vaa: None,
        }))
    }
}
