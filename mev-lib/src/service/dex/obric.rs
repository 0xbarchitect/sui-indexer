use crate::{
    constant,
    service::dex::DEXService,
    types::ObjectIDWrapper,
    utils::{self, ptb::PTBHelper},
};

use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{str::FromStr, sync::Arc};
use sui_sdk::{
    rpc_types::{Coin, SuiData, SuiObjectDataOptions},
    SuiClient,
};
use sui_types::base_types::{ObjectID, SuiAddress};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObricPool {
    pub id: ObjectIDWrapper,
    reserve_x: String,
    reserve_y: String,
    #[serde_as(as = "DisplayFromStr")]
    fee_millionth: u64,
}

pub struct ObricService {
    exchange: String,
    client: Arc<SuiClient>,
    pool_repo: Arc<dyn PoolRepository + Send + Sync>,
    coin_repo: Arc<dyn CoinRepository + Send + Sync>,
    ptb_helper: Arc<PTBHelper>,
}

impl ObricService {
    pub fn new(
        client: Arc<SuiClient>,
        pool_repo: Arc<dyn PoolRepository + Send + Sync>,
        coin_repo: Arc<dyn CoinRepository + Send + Sync>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        ObricService {
            exchange: constant::OBRIC_EXCHANGE.to_string(),
            client,
            pool_repo,
            coin_repo,
            ptb_helper,
        }
    }
}

#[async_trait]
impl DEXService for ObricService {
    /// Fetches the pool data from the Sui client using the provided pool ID.
    /// Returns a `Pool` struct containing the pool information.
    /// The function retrieves the pool type, coin types, and other relevant fields.
    async fn get_pool_data(&self, pool_id: &str) -> Result<crate::types::Pool> {
        let object_data_options = sui_sdk::rpc_types::SuiObjectDataOptions::full_content();

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

        // pool coins
        let pool_type = pool_data
            .type_
            .ok_or(anyhow!(
                "Failed to get object type for pool ID: {}",
                pool_id
            ))?
            .to_string();

        let coin_types = utils::get_coin_types_from_pool_type(&pool_type, &self.exchange)?;

        let coins: Vec<crate::types::Coin> =
            self.ptb_helper.fetch_coins_metadata(coin_types).await?;

        // pool fields

        let pool_fields = pool_data
            .content
            .ok_or_else(|| anyhow!("Missing object content"))?
            .try_into_move()
            .ok_or_else(|| anyhow!("Invalid move object"))?
            .fields;

        let pool_data =
            serde_json::from_value::<ObricPool>(pool_fields.to_json_value()).map_err(|e| {
                error!("Failed to deserialize pool fields: {}", e);
                e
            })?;
        info!("Obric deserialized: {:?}", pool_data);

        self.format_onchain_pool(&pool_data, coins)
    }
}

impl ObricService {
    fn format_onchain_pool(
        &self,
        pool: &ObricPool,
        coins: Vec<crate::types::Coin>,
    ) -> Result<crate::types::Pool> {
        let coin_amounts = vec![pool.reserve_x.clone(), pool.reserve_y.clone()];

        Ok(crate::types::Pool {
            exchange: self.exchange.clone(),
            pool_id: pool.id.id.to_string(),
            pool_type: None,
            coins,
            coin_amounts: Some(coin_amounts),
            weights: None,
            tick_spacing: None,
            current_tick_index: None,
            current_sqrt_price: None,
            liquidity: None,
            fee_rate: Some(pool.fee_millionth as i32),
            is_pause: None,
            fees_swap_in: None,
            fees_swap_out: None,
        })
    }
}
