use crate::types::FlashloanPool;

use anyhow::Result;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use toml;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DexConfig {
    pub package_id: Option<String>,
    pub integration_package_id: Option<String>,
    pub global_config_id: Option<String>,
    pub partner_id: Option<String>,
    pub aggregator_package_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LendingConfig {
    pub package_id: Option<String>,
    pub storage_id: Option<String>,
    pub api_endpoint: Option<String>,
    pub market_object_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub rpc_url: Option<String>,
    pub remote_store_url: Option<String>,
    pub ws_url: Option<String>,
    pub api_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CetusConfig {
    pub package_id: String,
    pub integration_package_id: String,
    pub global_config_id: String,
    pub partner_id: String,
    pub aggregator_package_id: String,
    pub aggregator_extend_package_id: String,
    pub aggregator_extend_v2_package_id: String,
    pub usdc_sui_pool_id: String,
    pub flash_loan_fee_rate: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AftermathConfig {
    pub pool_registry_id: String,
    pub protocol_fee_vault_id: String,
    pub treasury_id: String,
    pub insurance_fund_id: String,
    pub referal_vault_id: String,
    pub pool_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BluemoveConfig {
    pub dex_info_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObricConfig {
    pub pyth_state_object_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BluefinConfig {
    pub global_config_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlowXConfig {
    pub pool_registry_id: String,
    pub versioned_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TurbosConfig {
    pub versioned_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MomentumConfig {
    pub package_id: String,
    pub versioned_id: String,
    pub low_limit_price: String,
    pub high_limit_price: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NaviConfig {
    pub api_endpoint: String,
    pub package_id: String,
    pub storage_id: String,
    pub incentive_v2_id: String,
    pub incentive_v3_id: String,
    pub price_oracle_id: String,
    pub oracle_config_id: String,
    pub supra_oracle_holder_id: String,
    pub oracle_package_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuilendConfig {
    pub package_id: String,
    pub lending_market_id: String,
    pub lending_market_object_type: String,
    pub obligation_owner_cap_object_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScallopConfig {
    pub api_endpoint: String,
    pub package_id: String,
    pub versioned_id: String,
    pub market_id: String,
    pub coin_decimals_registry_id: String,
    pub obligation_key_object_type: String,
    pub xoracle_package_id: String,
    pub xoracle_object_id: String,
    pub xoracle_pyth_package_id: String,
    pub xoracle_pyth_state_id: String,
    pub xoracle_pyth_registry_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PythConfig {
    pub ws_url: String,
    pub api_url: String,
    pub package_id: String,
    pub wormhole_package_id: String,
    pub wormhole_state_id: String,
    pub pyth_state_id: String,
    pub price_identifier_type_tag: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArbitrageConfig {
    pub input_coin_type: String,
    pub input_amount: f64,
    pub max_path_length: usize,
    pub max_path_count: usize,

    pub coin_usd_liquidity_threshold: f64,
    pub max_dfs_adjacency_list_size: usize,
    pub slippage_tolerance: f64,
    pub reserve_pool_tolerance: f64,
    pub indexer_lagging_ms_threshold: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LiquidationConfig {
    pub borrower_debt_usd_threshold: f64,
    pub borrower_map_size: usize,
    pub hf_threshold_upper: f64,
    pub hf_threshold_lower: f64,
    pub gas_price: u64,
    pub gas_price_factor: f64,
    pub flashloan_threshold_usd: f64,
    pub flashloan_coin: String,
    pub flashloan_pool: FlashloanPool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexerConfig {
    pub dev_mode: bool,
    pub start_checkpoint_number: u64,
    pub local_checkpoint_dir: String,
    pub indexer_progress_filepath: String,
    pub use_remote_store: bool,
    pub indexer_worker_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub db_connection_pool_max_size: usize,
    pub db_connection_pool_idle_size: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // global
    pub log_level: String,
    pub run_mode: String,

    pub arbitrage_enabled: bool,
    pub liquidation_enabled: bool,
    pub onchain_indexer_enabled: bool,

    pub database: DatabaseConfig,
    pub networks: HashMap<String, NetworkConfig>,

    pub indexer: IndexerConfig,
    pub arbitrage: ArbitrageConfig,
    pub liquidation: LiquidationConfig,

    // dexes
    pub cetus: CetusConfig,
    pub aftermath: AftermathConfig,
    pub bluemove: BluemoveConfig,
    pub obric: ObricConfig,
    pub bluefin: BluefinConfig,
    pub flowx: FlowXConfig,
    pub turbos: TurbosConfig,
    pub momentum: MomentumConfig,

    // lendings
    pub navi: NaviConfig,
    pub suilend: SuilendConfig,
    pub scallop: ScallopConfig,

    // oracles
    pub pyth: PythConfig,
}

impl Config {
    pub fn load_toml() -> Result<Self> {
        let config_str = fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}
