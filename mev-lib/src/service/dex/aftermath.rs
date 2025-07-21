use crate::{
    constant,
    service::{self, dex::DEXService},
    types::ObjectIDWrapper,
    utils::{self, ptb::PTBHelper},
};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rust_decimal::{prelude::*, Decimal};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::sync::Arc;
use sui_sdk::rpc_types::{Coin, SuiData, SuiMoveValue, SuiObjectDataOptions};
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectID, SuiAddress};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AftermathPool {
    id: ObjectIDWrapper,
    type_names: Vec<String>,
    coin_decimals: Vec<u8>,
    normalized_balances: Vec<String>,
    decimal_scalars: Vec<String>,
    fees_swap_in: Vec<String>,
    fees_swap_out: Vec<String>,
    weights: Vec<String>,
    lp_decimal_scalar: String,
    lp_decimals: u8,
}

pub struct AftermathService {
    exchange: String,
    client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    ptb_helper: Arc<PTBHelper>,
}

impl AftermathService {
    pub fn new(
        client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        AftermathService {
            exchange: constant::AFTERMATH_EXCHANGE.to_string(),
            client,
            pool_repo,
            coin_repo,
            ptb_helper,
        }
    }
}

#[async_trait]
impl DEXService for AftermathService {
    /// Fetches the pool data from the Sui client using the provided pool ID.
    /// Returns a `Pool` struct containing the pool information.
    ///
    async fn get_pool_data(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let object_data_options = SuiObjectDataOptions::full_content();

        let pool_id = ObjectID::from_str(pool_id)?;

        let pool_obj = self
            .client
            .read_api()
            .get_object_with_options(pool_id, object_data_options)
            .await?;

        let pool_data = pool_obj.data.ok_or(anyhow!(
            "Failed to get object data for pool ID: {}",
            pool_id
        ))?;

        // pool type
        let pool_type = pool_data
            .type_
            .ok_or(anyhow!(
                "Failed to get object type for pool ID: {}",
                pool_id
            ))?
            .to_string();

        let pool_type = utils::extract_pool_type(&pool_type, &self.exchange)?;

        // pool data
        let pool_fields = pool_data
            .content
            .ok_or_else(|| anyhow!("Missing object content"))?
            .try_into_move()
            .ok_or_else(|| anyhow!("Invalid move object"))?
            .fields;

        let pool_data = serde_json::from_value::<AftermathPool>(pool_fields.to_json_value())
            .map_err(|e| {
                error!("Failed to deserialize pool fields: {}", e);
                e
            })?;
        info!("Aftermath deserialized: {:?}", pool_data);

        self.format_onchain_pool(&pool_data, &pool_type)
    }
}

impl AftermathService {
    fn format_onchain_pool(
        &self,
        pool: &AftermathPool,
        pool_type: &str,
    ) -> Result<crate::types::Pool> {
        let coins = pool
            .type_names
            .iter()
            .map(|coin_type| utils::format_type_name(coin_type, true))
            .collect::<Vec<String>>()
            .iter()
            .zip(pool.coin_decimals.iter())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|c| crate::types::Coin {
                coin_type: c.0.clone(),
                decimals: *c.1,
                name: None,
                symbol: None,
                pyth_feed_id: None,
                pyth_info_object_id: None,
            })
            .collect::<Vec<_>>();

        let coin_amounts = pool
            .normalized_balances
            .iter()
            .zip(pool.decimal_scalars.iter())
            .map(|(amount, decimal)| {
                let Some(amount) = Decimal::from_str(amount).ok() else {
                    return Decimal::ZERO;
                };
                let Some(decimal) = Decimal::from_str(decimal).ok() else {
                    return Decimal::ZERO;
                };
                if decimal.is_zero() {
                    return Decimal::ZERO;
                }
                amount / decimal
            })
            .collect::<Vec<_>>()
            .iter()
            .map(Decimal::to_string)
            .collect::<Vec<String>>();

        let weights = pool
            .weights
            .iter()
            .zip(pool.decimal_scalars.iter())
            .map(|(weight, decimal)| {
                let Some(weight) = Decimal::from_str(weight).ok() else {
                    return Decimal::ZERO;
                };
                let Some(decimal) = Decimal::from_str(decimal).ok() else {
                    return Decimal::ZERO;
                };
                if decimal.is_zero() {
                    return Decimal::ZERO;
                }
                weight / decimal
            })
            .collect::<Vec<_>>()
            .iter()
            .map(Decimal::to_string)
            .collect::<Vec<String>>();

        let liquidity = pool
            .lp_decimal_scalar
            .parse::<Decimal>()
            .unwrap_or(Decimal::ZERO);

        let liquidity = if pool.lp_decimals == 0 {
            Decimal::ZERO
        } else {
            liquidity / Decimal::from(10).powu(pool.lp_decimals as u64)
        };

        let fees_swap_in = pool
            .fees_swap_in
            .iter()
            .zip(pool.decimal_scalars.iter())
            .map(|(fee, decimal)| {
                let Some(fee) = Decimal::from_str(fee).ok() else {
                    return Decimal::ZERO;
                };
                let Some(decimal) = Decimal::from_str(decimal).ok() else {
                    return Decimal::ZERO;
                };
                if decimal.is_zero() {
                    return Decimal::ZERO;
                }
                fee / decimal
            })
            .collect::<Vec<_>>()
            .iter()
            .map(Decimal::to_string)
            .collect::<Vec<String>>();

        let fees_swap_out = pool
            .fees_swap_out
            .iter()
            .zip(pool.decimal_scalars.iter())
            .map(|(fee, decimal)| {
                let Some(fee) = Decimal::from_str(fee).ok() else {
                    return Decimal::ZERO;
                };
                let Some(decimal) = Decimal::from_str(decimal).ok() else {
                    return Decimal::ZERO;
                };
                if decimal.is_zero() {
                    return Decimal::ZERO;
                }
                fee / decimal
            })
            .collect::<Vec<_>>()
            .iter()
            .map(Decimal::to_string)
            .collect::<Vec<String>>();

        Ok(crate::types::Pool {
            exchange: self.exchange.clone(),
            pool_id: pool.id.id.to_string(),
            pool_type: Some(pool_type.to_string()),
            coins,
            coin_amounts: Some(coin_amounts),
            weights: Some(weights),
            tick_spacing: None,
            current_tick_index: None,
            current_sqrt_price: None,
            liquidity: Some(liquidity.to_string()),
            fee_rate: None,
            is_pause: Some(false),
            fees_swap_in: Some(fees_swap_in),
            fees_swap_out: Some(fees_swap_out),
        })
    }
}
