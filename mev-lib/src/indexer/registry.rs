use crate::{
    config::Config,
    constant,
    indexer::{dex, lending, oracle, EventProcessor, OnchainEvent},
    service::{
        db_service::{lending::LendingService, pool::PoolService},
        registry::ServiceRegistry,
    },
    types::Borrower,
    utils,
};
use db::{
    models,
    repositories::{CoinRepository, PoolRepository, UserBorrowRepository, UserDepositRepository},
};

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::{path::Path, str::FromStr, sync::Arc};
use sui_json_rpc_types::SuiEvent;
use sui_sdk::SuiClient;
use sui_types::event::{self, Event};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, trace, warn};

pub struct EventProcessorRegistry {
    config: Arc<Config>,
    db_pool_service: Arc<PoolService>,
    db_lending_service: Arc<LendingService>,
    service_registry: Arc<ServiceRegistry>,
    dex_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>>,
    lending_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>>,
    oracle_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>>,
}

impl EventProcessorRegistry {
    pub fn new(
        config: Arc<Config>,
        client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        db_pool_service: Arc<PoolService>,
        db_lending_service: Arc<LendingService>,
        service_registry: Arc<ServiceRegistry>,
    ) -> Self {
        let mut dex_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>> =
            HashMap::new();
        let mut lending_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>> =
            HashMap::new();
        let mut oracle_processors: HashMap<String, Arc<dyn EventProcessor + Send + Sync>> =
            HashMap::new();

        let navi_config = Arc::new(config.navi.clone());
        let scallop_config = Arc::new(config.scallop.clone());
        let suilend_config = Arc::new(config.suilend.clone());

        // services
        let cetus_service = service_registry
            .get_dex_service(constant::CETUS_EXCHANGE)
            .unwrap();

        let bluefin_service = service_registry
            .get_dex_service(constant::BLUEFIN_EXCHANGE)
            .unwrap();

        let turbos_service = service_registry
            .get_dex_service(constant::TURBOS_EXCHANGE)
            .unwrap();

        let momentum_service = service_registry
            .get_dex_service(constant::MOMENTUM_EXCHANGE)
            .unwrap();

        let aftermath_service = service_registry
            .get_dex_service(constant::AFTERMATH_EXCHANGE)
            .unwrap();

        let flowx_service = service_registry
            .get_dex_service(constant::FLOWX_EXCHANGE)
            .unwrap();

        let bluemove_service = service_registry
            .get_dex_service(constant::BLUEMOVE_EXCHANGE)
            .unwrap();

        let obric_service = service_registry
            .get_dex_service(constant::OBRIC_EXCHANGE)
            .unwrap();

        let navi_service = service_registry
            .get_lending_service(constant::NAVI_LENDING)
            .unwrap();

        let scallop_service = service_registry
            .get_lending_service(constant::SCALLOP_LENDING)
            .unwrap();

        let suilend_service = service_registry
            .get_lending_service(constant::SUILEND_LENDING)
            .unwrap();

        // dex processors
        let cetus_processor = Arc::new(dex::cetus::Cetus::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&cetus_service),
        ));

        let bluefin_processor = Arc::new(dex::bluefin::Bluefin::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&bluefin_service),
        ));

        let turbos_processor = Arc::new(dex::turbos::Turbos::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&turbos_service),
        ));

        let momentum_processor = Arc::new(dex::momentum::Momentum::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&momentum_service),
        ));

        let aftermath_processor = Arc::new(dex::aftermath::Aftermath::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&aftermath_service),
        ));

        let flowx_processor = Arc::new(dex::flowx::FlowX::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&flowx_service),
        ));

        let bluemove_processor = Arc::new(dex::bluemove::Bluemove::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&bluemove_service),
        ));

        let obric_processor = Arc::new(dex::obric::Obric::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&obric_service),
        ));

        // lending processors
        let navi_processor = Arc::new(lending::navi::Navi::new(
            Arc::clone(&client),
            Arc::clone(&navi_config),
            Arc::clone(&navi_service),
            Arc::clone(&db_lending_service),
        ));

        let suilend_processor = Arc::new(lending::suilend::SuiLend::new(
            Arc::clone(&client),
            Arc::clone(&suilend_config),
            Arc::clone(&suilend_service),
            Arc::clone(&db_lending_service),
        ));

        let scallop_processor = Arc::new(lending::scallop::Scallop::new(
            Arc::clone(&client),
            Arc::clone(&scallop_config),
            Arc::clone(&scallop_service),
            Arc::clone(&db_lending_service),
        ));

        // oracle processors
        let pyth_processor = Arc::new(oracle::pyth::Pyth::new(
            Arc::clone(&client),
            Arc::clone(&coin_repo),
            Arc::clone(&db_pool_service),
            Arc::clone(&db_lending_service),
        ));

        // dexs
        if config.arbitrage_enabled {
            dex_processors.insert(
                constant::CETUS_SWAP_EVENT.to_string(),
                Arc::clone(&cetus_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::CETUS_ADD_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&cetus_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::CETUS_REMOVE_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&cetus_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::BLUEFIN_SWAP_EVENT.to_string(),
                Arc::clone(&bluefin_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::BLUEFIN_TICK_UPDATED_EVENT.to_string(),
                Arc::clone(&bluefin_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::TURBOS_SWAP_EVENT.to_string(),
                Arc::clone(&turbos_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::TURBOS_ADD_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&turbos_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::TURBOS_REMOVE_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&turbos_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::MOMENTUM_SWAP_EVENT.to_string(),
                Arc::clone(&momentum_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::MOMENTUM_ADD_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&momentum_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::MOMENTUM_REMOVE_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&momentum_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::FLOWX_SWAP_EVENT.to_string(),
                Arc::clone(&flowx_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::FLOWX_MODIFY_LIQUIDITY_EVENT.to_string(),
                Arc::clone(&flowx_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::BLUEMOVE_SWAP_EVENT.to_string(),
                Arc::clone(&bluemove_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::AFTERMATH_SWAP_EVENT.to_string(),
                Arc::clone(&aftermath_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            dex_processors.insert(
                constant::OBRIC_SWAP_EVENT.to_string(),
                Arc::clone(&obric_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );
        }

        // lendings
        if config.liquidation_enabled {
            // navi
            lending_processors.insert(
                constant::NAVI_BORROW_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::NAVI_DEPOSIT_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::NAVI_REPAY_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::NAVI_WITHDRAW_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::NAVI_LIQUIDATE_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::NAVI_STATE_UPDATED_EVENT.to_string(),
                Arc::clone(&navi_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            // suilend
            lending_processors.insert(
                constant::SUILEND_BORROW_EVENT.to_string(),
                Arc::clone(&suilend_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SUILEND_DEPOSIT_EVENT.to_string(),
                Arc::clone(&suilend_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SUILEND_REPAY_EVENT.to_string(),
                Arc::clone(&suilend_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SUILEND_WITHDRAW_EVENT.to_string(),
                Arc::clone(&suilend_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SUILEND_LIQUIDATE_EVENT.to_string(),
                Arc::clone(&suilend_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            // scallop
            lending_processors.insert(
                constant::SCALLOP_BORROW_EVENT.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_BORROW_EVENT_V2.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_BORROW_EVENT_V3.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_DEPOSIT_EVENT.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_REPAY_EVENT.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_WITHDRAW_EVENT.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );

            lending_processors.insert(
                constant::SCALLOP_LIQUIDATE_EVENT_V2.to_string(),
                Arc::clone(&scallop_processor) as Arc<dyn EventProcessor + Send + Sync>,
            );
        }

        // oracles
        oracle_processors.insert(
            constant::PYTH_UPDATE_PRICE_EVENT.to_string(),
            Arc::clone(&pyth_processor) as Arc<dyn EventProcessor + Send + Sync>,
        );

        Self {
            config,
            db_pool_service,
            db_lending_service,
            service_registry,
            dex_processors,
            lending_processors,
            oracle_processors,
        }
    }

    /// Processes tx events.
    /// Mostly for development purposes.
    ///
    pub async fn process_tx_event(&self, event: SuiEvent, tx_digest: &str) -> Result<()> {
        let event_type = utils::extract_event_type(&event.type_.to_string())?;
        let sender = event.sender.to_string();
        let data = event.parsed_json;

        if let Some(processor) = self.find_processor_for_event_type(&event_type) {
            processor
                .process_tx_event(&event_type, &sender, data, tx_digest)
                .await
        } else {
            Err(anyhow!("No processor found for event type: {}", event_type))
        }
    }

    /// Processes raw event from checkpoint data.
    ///
    pub async fn process_raw_event(&self, event: Event, tx_digest: &str) -> Result<OnchainEvent> {
        let event_type = utils::extract_event_type(&event.type_.to_string())?;
        let sender = event.sender.to_string();

        if let Some(processor) = self.find_processor_for_event_type(&event_type) {
            processor
                .process_raw_event(&event_type, &sender, event, tx_digest)
                .await
                .map_err(|e| {
                    anyhow!(
                        "{} failed to process event {}: {}",
                        processor,
                        event_type,
                        e
                    )
                })
        } else {
            Err(anyhow!(
                "No processor found for event type: {}",
                &event_type
            ))
        }
    }

    /// Retrieves the event ID based on the event type and data.
    /// This ID is used to identify the event across checkpoints events.
    /// E.g: the swap event of a pool is identified by the event type and the pool ID.
    /// By identifying the event, we can select to process only the latest event,
    /// ignoring all the previous events occured on the same entity (pool, obligation, price feed)
    pub fn get_event_id(&self, event: &Event) -> Result<String> {
        let event_type = utils::extract_event_type(&event.type_.to_string())?;

        if let Some(processor) = self.find_processor_for_event_type(&event_type) {
            processor.get_event_id(&event_type, event).map_err(|e| {
                error!(
                    "{} failed to get event ID for event type {}: {}",
                    processor, event_type, e
                );
                anyhow!(
                    "{} failed to get event ID for event type {}: {}",
                    processor,
                    event_type,
                    e
                )
            })
        } else {
            Err(anyhow!("No processor found for event type: {}", event_type))
        }
    }

    /// Finds the appropriate processor for the given event type.
    ///
    fn find_processor_for_event_type(
        &self,
        event_type: &str,
    ) -> Option<Arc<dyn EventProcessor + Send + Sync>> {
        if self.config.arbitrage_enabled {
            // Check if the event type is in the dex processors
            if let Some(processor) = self.dex_processors.get(event_type) {
                return Some(processor.clone());
            }
        }

        if self.config.liquidation_enabled {
            // Check if the event type is in the lending processors
            if let Some(processor) = self.lending_processors.get(event_type) {
                return Some(processor.clone());
            }
        }

        // oracle processors is always enabled
        if let Some(processor) = self.oracle_processors.get(event_type) {
            return Some(processor.clone());
        }

        None
    }
}
