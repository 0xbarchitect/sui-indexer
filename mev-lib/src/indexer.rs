pub mod dex;
pub mod lending;
pub mod onchain_indexer;
pub mod oracle;
pub mod registry;

use crate::{
    config::Config,
    constant,
    service::{
        db_service::{lending::LendingService, pool::PoolService},
        registry::ServiceRegistry,
    },
    utils,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{self, NewPool, Pool, UpdatePool},
};
use db::repositories::{
    CoinRepository, PoolRepository, UserBorrowRepository, UserDepositRepository,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::Display;
use sui_types::event::{self, Event};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnchainEvent {
    DEXSwap(DEXSwapEvent),
    DEXLiquidity(DEXLiquidityEvent),
    LendingDeposit(lending::DepositEvent),
    LendingWithdraw(lending::WithdrawEvent),
    LendingBorrow(lending::BorrowEvent),
    LendingRepay(lending::RepayEvent),
    LendingLiquidate(lending::LiquidateEvent),
    LendingIndexUpdated(lending::IndexUpdatedEvent),
    OraclePrice(OraclePriceEvent),
    VoidEvent, // this is used to indicate that the event should not be processed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DEXSwapEvent {
    pub exchange: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DEXLiquidityEvent {
    pub exchange: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OraclePriceEvent {
    pub oracle: String,
    pub feed_id: String,
    pub spot_price: String,
    pub ema_price: String,
    pub publish_time: u64,
    pub vaa: Option<String>,
}

impl Display for OraclePriceEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OraclePriceEvent {{ oracle: {}, feed_id: {}, spot_price: {}, ema_price: {}, publish_time: {} }}",
            self.oracle, self.feed_id, self.spot_price, self.ema_price, self.publish_time,
        )
    }
}

#[async_trait]
pub trait EventProcessor: Display {
    /// Process an event in transaction data.
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()>;

    /// Process a raw event in checkpoint data.
    async fn process_raw_event(
        &self,
        event_type: &str,
        sender: &str,
        event: sui_types::event::Event,
        tx_digest: &str,
    ) -> Result<OnchainEvent>;

    /// Retrieves the event ID based event data.
    /// This ID is used to identify the event across checkpoints events.
    /// E.g: the swap event of a pool is identified by the pool ID.
    /// By identifying the event, we can select to process only the latest event,
    /// ignoring all the previous events occured on the same entity (pool, obligation, price feed)
    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String>;
}
