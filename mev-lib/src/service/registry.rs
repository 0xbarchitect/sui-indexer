use crate::{
    config::Config,
    constant,
    service::{db_service, dex, lending},
    utils::ptb::PTBHelper,
};
use db::{
    models,
    repositories::{CoinRepository, LendingMarketRepository, PoolRepository},
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rust_decimal::{prelude::*, Decimal};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Write, path::Path, sync::Arc};
use sui_sdk::SuiClient;
use sui_types::coin;
use tokio::time::{Duration, Instant};
use toml;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BorrowerList {
    pub borrowers: Vec<BorrowerEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BorrowerEntry {
    pub platform: String,
    pub borrower: String,
}

pub struct ServiceRegistry {
    pub config: Arc<Config>,
    pub coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    pub pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    pub lending_market_repo: Arc<dyn LendingMarketRepository + Send + Sync>,
    pub db_pool_service: Arc<db_service::pool::PoolService>,
    pub db_lending_service: Arc<db_service::lending::LendingService>,
    pub ptb_helper: Arc<PTBHelper>,

    pub dexes: HashMap<String, Arc<dyn dex::DEXService + Send + Sync>>,
    pub lendings: HashMap<String, Arc<dyn lending::LendingService + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new(
        config: Arc<Config>,
        client: Arc<SuiClient>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        lending_market_repo: Arc<dyn LendingMarketRepository + Send + Sync>,
        db_pool_service: Arc<db_service::pool::PoolService>,
        db_lending_service: Arc<db_service::lending::LendingService>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        let mut dexes = HashMap::new();
        let mut lendings = HashMap::new();

        // Initialize DEX services
        let cetus_service = Arc::new(dex::cetus::CetusService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::CETUS_EXCHANGE.to_string(),
            Arc::clone(&cetus_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let aftermath_service = Arc::new(dex::aftermath::AftermathService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::AFTERMATH_EXCHANGE.to_string(),
            Arc::clone(&aftermath_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let momentum_service = Arc::new(dex::momentum::MomentumService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::MOMENTUM_EXCHANGE.to_string(),
            Arc::clone(&momentum_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let obric_service = Arc::new(dex::obric::ObricService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::OBRIC_EXCHANGE.to_string(),
            Arc::clone(&obric_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let bluefin_service = Arc::new(dex::bluefin::BluefinService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::BLUEFIN_EXCHANGE.to_string(),
            Arc::clone(&bluefin_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let bluemove_service = Arc::new(dex::bluemove::BluemoveService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::BLUEMOVE_EXCHANGE.to_string(),
            Arc::clone(&bluemove_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let turbos_service = Arc::new(dex::turbos::TurbosService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::TURBOS_EXCHANGE.to_string(),
            Arc::clone(&turbos_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        let flowx_service = Arc::new(dex::flowx::FlowXService::new(
            Arc::clone(&client),
            Arc::clone(&pool_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&ptb_helper),
        ));

        dexes.insert(
            constant::FLOWX_EXCHANGE.to_string(),
            Arc::clone(&flowx_service) as Arc<dyn dex::DEXService + Send + Sync>,
        );

        // Initialize Lending services
        let navi_config = Arc::new(config.navi.clone());
        let suilend_config = Arc::new(config.suilend.clone());
        let scallop_config = Arc::new(config.scallop.clone());

        let navi_service = Arc::new(lending::navi::NaviService::new(
            Arc::clone(&navi_config),
            Arc::clone(&client),
            Arc::clone(&lending_market_repo),
            Arc::clone(&coin_repo),
            Arc::clone(&db_lending_service),
            Arc::clone(&ptb_helper),
        ));

        lendings.insert(
            constant::NAVI_LENDING.to_string(),
            Arc::clone(&navi_service) as Arc<dyn lending::LendingService + Send + Sync>,
        );

        let suilend_service = Arc::new(lending::suilend::SuilendService::new(
            Arc::clone(&suilend_config),
            Arc::clone(&client),
            Arc::clone(&db_lending_service),
            Arc::clone(&ptb_helper),
        ));

        lendings.insert(
            constant::SUILEND_LENDING.to_string(),
            Arc::clone(&suilend_service) as Arc<dyn lending::LendingService + Send + Sync>,
        );

        let scallop_service = Arc::new(lending::scallop::ScallopService::new(
            Arc::clone(&scallop_config),
            Arc::clone(&client),
            Arc::clone(&db_pool_service),
            Arc::clone(&db_lending_service),
            Arc::clone(&ptb_helper),
        ));

        lendings.insert(
            constant::SCALLOP_LENDING.to_string(),
            Arc::clone(&scallop_service) as Arc<dyn lending::LendingService + Send + Sync>,
        );

        ServiceRegistry {
            config,
            coin_repo,
            pool_repo,
            lending_market_repo,
            db_pool_service,
            db_lending_service,
            ptb_helper,
            dexes,
            lendings,
        }
    }

    pub fn get_dex_service(&self, name: &str) -> Result<Arc<dyn dex::DEXService + Send + Sync>> {
        self.dexes
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("DEX service not found: {}", name))
    }

    pub fn get_lending_service(
        &self,
        name: &str,
    ) -> Result<Arc<dyn lending::LendingService + Send + Sync>> {
        self.lendings
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Lending service not found: {}", name))
    }
}
