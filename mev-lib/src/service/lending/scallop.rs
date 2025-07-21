use crate::{
    config::ScallopConfig,
    constant,
    service::{db_service, lending::LendingService},
    types::{FixedPoint32, FixedPoint32Json, ObjectIDWrapper, TypeName},
    utils::{self, ptb::PTBHelper},
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use reqwest;
use rust_decimal::{prelude::*, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashSet, path::Path, str::FromStr, sync::Arc};
use sui_sdk::{
    rpc_types::{Coin, SuiData, SuiMoveValue, SuiObjectData, SuiObjectDataOptions},
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, SequenceNumber, SuiAddress},
    dynamic_field::DynamicFieldName,
    event::Event,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Command, ObjectArg, TransactionData, TransactionKind},
    Identifier, TypeTag,
};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScallopMarket {
    pub interest_models: InterestModelTable,
    pub risk_models: RiskModelTable,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterestModelTable {
    pub table: InterestModelTableID,
    pub keys: InterestModelKeys,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterestModelTableID {
    pub id: ObjectIDWrapper,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterestModelKeys {
    pub contents: Vec<TypeName>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskModelTable {
    pub table: RistModelTableID,
    pub keys: RiskModelKeys,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RistModelTableID {
    pub id: ObjectIDWrapper,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskModelKeys {
    pub contents: Vec<TypeName>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterestModelDynamicField {
    pub name: TypeName,
    pub value: InterestModelJson,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterestModelJson {
    pub base_borrow_rate_per_sec: FixedPoint32Json,
    #[serde_as(as = "DisplayFromStr")]
    pub interest_rate_scale: u64,
    pub borrow_rate_on_mid_kink: FixedPoint32Json,
    pub mid_kink: FixedPoint32Json,
    pub borrow_rate_on_high_kink: FixedPoint32Json,
    pub high_kink: FixedPoint32Json,
    pub max_borrow_rate: FixedPoint32Json,
    pub revenue_factor: FixedPoint32Json,
    pub borrow_weight: FixedPoint32Json,
    #[serde_as(as = "DisplayFromStr")]
    pub min_borrow_amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskModelDynamicField {
    pub name: TypeName,
    pub value: RiskModelJson,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskModelJson {
    pub collateral_factor: FixedPoint32Json,
    pub liquidation_factor: FixedPoint32Json,
    pub liquidation_penalty: FixedPoint32Json,
    pub liquidation_discount: FixedPoint32Json,
    pub liquidation_revenue_factor: FixedPoint32Json,
    #[serde_as(as = "DisplayFromStr")]
    pub max_collateral_amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BorrowerAsset {
    pub coin_type: String,
    pub amount: u64,
    pub debt_borrow_index: Option<u64>,
    pub is_collateral: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiCollateralResponse {
    pub collaterals: Vec<Collateral>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Collateral {
    #[serde(rename = "coinType")]
    pub coin_type: String,
    #[serde(rename = "collateralFactor")]
    pub collateral_factor: f64,
    #[serde(rename = "liquidationFactor")]
    pub liquidation_factor: f64,
    #[serde(rename = "liquidationDiscount")]
    pub liquidation_discount: f64,
    #[serde(rename = "liquidationPenalty")]
    pub liquidation_penalty: f64,
    #[serde(rename = "liquidationReserveFactor")]
    pub liquidation_reserve_factor: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiPoolResponse {
    pub pools: Vec<LendingPool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LendingPool {
    #[serde(rename = "coinType")]
    pub coin_type: String,
    #[serde(rename = "borrowWeight")]
    pub borrow_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiquidationFactor {
    pub ltv: String,
    pub liquidation_threshold: String,
    pub liquidation_penalty: String,
    pub liquidation_fee: String,
}

pub struct ScallopService {
    platform: String,
    config: Arc<ScallopConfig>,
    client: Arc<SuiClient>,
    db_pool_service: Arc<db_service::pool::PoolService>,
    db_lending_service: Arc<db_service::lending::LendingService>,
    ptb_helper: Arc<PTBHelper>,
}

impl ScallopService {
    pub fn new(
        config: Arc<ScallopConfig>,
        client: Arc<SuiClient>,
        db_pool_service: Arc<db_service::pool::PoolService>,
        db_lending_service: Arc<db_service::lending::LendingService>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        ScallopService {
            platform: constant::SCALLOP_LENDING.to_string(),
            config,
            client,
            db_pool_service,
            db_lending_service,
            ptb_helper,
        }
    }
}

#[async_trait]
impl LendingService for ScallopService {
    /// Fetches the borrower portfolio from on-chain data.
    ///
    async fn fetch_borrower_portfolio(
        &self,
        borrower: String,
        obligation_id: Option<String>,
    ) -> Result<(
        Vec<crate::types::UserDeposit>,
        Vec<crate::types::UserBorrow>,
    )> {
        // retrieve obligation id
        let obligation_id = self.find_obligation_id_from_address(&borrower).await?;

        info!(
            "Found obligation ID: {} for borrower {}",
            &obligation_id, &borrower
        );

        self.process_obligation(&borrower, obligation_id).await
    }

    async fn fetch_user_deposit(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserDeposit> {
        let obligation_id = obligation_id
            .ok_or_else(|| anyhow!("Obligation ID is required for fetching user deposit"))?;
        let coin_type =
            coin_type.ok_or_else(|| anyhow!("Coin type is required for fetching user deposit"))?;

        let borrower_asset = self
            .fetch_asset_amount(&obligation_id, &coin_type, true)
            .await?;

        Ok(crate::types::UserDeposit {
            platform: self.platform.clone(),
            borrower,
            coin_type: borrower_asset.coin_type,
            amount: borrower_asset.amount.to_string(),
            obligation_id: Some(obligation_id),
        })
    }

    async fn fetch_user_borrow(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        asset_id: Option<u8>,
    ) -> Result<crate::types::UserBorrow> {
        let obligation_id = obligation_id
            .ok_or_else(|| anyhow!("Obligation ID is required for fetching user borrow"))?;
        let coin_type =
            coin_type.ok_or_else(|| anyhow!("Coin type is required for fetching user borrow"))?;

        let borrower_asset = self
            .fetch_asset_amount(&obligation_id, &coin_type, false)
            .await?;

        Ok(crate::types::UserBorrow {
            platform: self.platform.clone(),
            borrower,
            coin_type: borrower_asset.coin_type,
            amount: borrower_asset.amount.to_string(),
            obligation_id: Some(obligation_id),
            debt_borrow_index: borrower_asset.debt_borrow_index.map(|b| b.to_string()),
        })
    }

    async fn find_obligation_id_from_address(&self, borrower: &str) -> Result<String> {
        let obligation_keys = self
            .ptb_helper
            .find_owned_objects_given_owner_address_and_type(
                SuiAddress::from_str(borrower)?,
                &self.config.obligation_key_object_type,
                true,
            )
            .await?;

        if obligation_keys.is_empty() {
            return Err(anyhow!(
                "No obligation keys found for borrower: {}",
                borrower
            ));
        }

        let fields = obligation_keys[0]
            .clone()
            .content
            .ok_or_else(|| anyhow!("Missing object content"))?
            .try_into_move()
            .ok_or_else(|| anyhow!("Invalid move object"))?
            .fields;

        let obligation_id = match fields.field_value("ownership") {
            Some(SuiMoveValue::Struct(v)) => v
                .field_value("of")
                .ok_or(anyhow!("Missing of field"))?
                .to_string(),
            _ => return Err(anyhow!("Invalid ownership field")),
        };

        Ok(obligation_id)
    }
}

impl ScallopService {
    /// Processes a single obligation for a borrower.
    /// Returns a tuple containing vectors of user deposits and user borrows.
    ///
    async fn process_obligation(
        &self,
        borrower: &str,
        obligation_id: String,
    ) -> Result<(
        Vec<crate::types::UserDeposit>,
        Vec<crate::types::UserBorrow>,
    )> {
        info!("Process obligation: {}", &obligation_id);

        // fetch user collateral and debt asset types
        let collateral_types = self
            .get_asset_types(&obligation_id, true)
            .await
            .map_err(|e| anyhow!("Failed to get collateral types: {}", e))?;

        let debt_types = self
            .get_asset_types(&obligation_id, false)
            .await
            .map_err(|e| anyhow!("Failed to get debt types: {}", e))?;

        let collateral_assets = stream::iter(collateral_types)
            .map(|asset_type| {
                let obligation_id = obligation_id.clone();
                async move {
                    self.fetch_asset_amount(&obligation_id, &asset_type, true)
                        .await
                }
            })
            .buffered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let debt_assets = stream::iter(debt_types)
            .map(|asset_type| {
                let obligation_id = obligation_id.clone();
                async move {
                    self.fetch_asset_amount(&obligation_id, &asset_type, false)
                        .await
                }
            })
            .buffered(10)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let user_deposits = collateral_assets
            .iter()
            .map(|asset| crate::types::UserDeposit {
                platform: self.platform.clone(),
                borrower: borrower.to_string(),
                coin_type: asset.coin_type.clone(),
                amount: asset.amount.to_string(),
                obligation_id: Some(obligation_id.clone()),
            })
            .collect::<Vec<_>>();

        let user_borrows = debt_assets
            .iter()
            .map(|asset| crate::types::UserBorrow {
                platform: self.platform.clone(),
                borrower: borrower.to_string(),
                coin_type: asset.coin_type.clone(),
                amount: asset.amount.to_string(),
                obligation_id: Some(obligation_id.clone()),
                debt_borrow_index: asset.debt_borrow_index.map(|b| b.to_string()),
            })
            .collect::<Vec<_>>();

        Ok((user_deposits, user_borrows))
    }

    /// Fetch borrower collateral and debt asset types.
    /// by calling smart contract functions.
    ///
    async fn get_asset_types(
        &self,
        obligation_id: &str,
        is_collateral: bool,
    ) -> Result<Vec<String>> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let obligation_arg = ptb.obj(
            self.ptb_helper
                .build_shared_obj_arg(obligation_id, false)
                .await?,
        )?;

        let function = if is_collateral {
            Identifier::new("collateral_types")?
        } else {
            Identifier::new("debt_types")?
        };

        ptb.command(Command::move_call(
            ObjectID::from_str(&self.config.package_id)?,
            Identifier::new("obligation")?,
            function,
            vec![],
            vec![obligation_arg],
        ));

        let builder = ptb.finish();
        let tx = TransactionKind::ProgrammableTransaction(builder);
        let response = self
            .client
            .read_api()
            .dev_inspect_transaction_block(SuiAddress::default(), tx, None, None, None)
            .await?;
        let results = response.results.ok_or(anyhow!(
            "Failed to get return values from dev_inspect_transaction_block"
        ))?;

        if is_collateral {
            debug!("collateral types results : {:?}", results);
        } else {
            debug!("debt types results : {:?}", results);
        }

        let asset_types = results
            .first()
            .ok_or(anyhow!(
                "No return values found in dev_inspect_transaction_block"
            ))?
            .return_values
            .first()
            .ok_or(anyhow!(
                "No return values found in dev_inspect_transaction_block"
            ))?;
        let asset_types = bcs::from_bytes::<Vec<String>>(&asset_types.0)
            .map_err(|e| anyhow!("Failed to deserialize return value: {}", e))?;

        Ok(asset_types)
    }

    /// Fetches the amount of a specific asset type for a borrower.
    /// by calling smart contract functions.
    ///
    async fn fetch_asset_amount(
        &self,
        obligation_id: &str,
        asset_type: &str,
        is_collateral: bool,
    ) -> Result<BorrowerAsset> {
        let start = Instant::now();

        let mut ptb = ProgrammableTransactionBuilder::new();
        let obligation_arg = ptb.obj(
            self.ptb_helper
                .build_shared_obj_arg(obligation_id, false)
                .await?,
        )?;

        let asset_arg = ptb.pure_bytes(bcs::to_bytes(asset_type)?, false);

        let function = if is_collateral {
            Identifier::new("collateral")?
        } else {
            Identifier::new("debt")?
        };

        ptb.command(Command::move_call(
            ObjectID::from_str(&self.config.package_id)?,
            Identifier::new("obligation")?,
            function,
            vec![],
            vec![obligation_arg, asset_arg],
        ));

        let builder = ptb.finish();
        let tx = TransactionKind::ProgrammableTransaction(builder);
        let response = self
            .client
            .read_api()
            .dev_inspect_transaction_block(SuiAddress::default(), tx, None, None, None)
            .await?;
        let results = response.results.ok_or(anyhow!(
            "Failed to get return values from dev_inspect_transaction_block"
        ))?;

        let return_values = &results
            .first()
            .ok_or(anyhow!(
                "No return values found in dev_inspect_transaction_block"
            ))?
            .return_values;

        let amount = return_values.first().ok_or(anyhow!(
            "No return values found in dev_inspect_transaction_block"
        ))?;

        let amount = bcs::from_bytes::<u64>(&amount.0)
            .map_err(|e| anyhow!("Failed to deserialize amount value: {}", e))?;

        let debt_borrow_index = if is_collateral {
            None
        } else {
            let b_index = return_values.get(1).ok_or(anyhow!(
                "No borrow index value found in dev_inspect_transaction_block"
            ))?;

            let b_index = bcs::from_bytes::<u64>(&b_index.0)
                .map_err(|e| anyhow!("Failed to deserialize borrow_index value: {}", e))?;

            Some(b_index)
        };

        let elapsed = start.elapsed();
        info!(
            "asset_amount response: asset {}, amount {}, borrow_index {:?} in {:?}ms",
            asset_type,
            amount,
            debt_borrow_index,
            elapsed.as_millis()
        );

        Ok(BorrowerAsset {
            coin_type: utils::format_type_name(asset_type, true),
            amount,
            debt_borrow_index,
            is_collateral,
        })
    }
}
