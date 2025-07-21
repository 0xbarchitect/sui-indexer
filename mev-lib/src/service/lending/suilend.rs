use crate::{
    config::SuilendConfig,
    constant, indexer,
    service::{db_service, lending::LendingService},
    types::{ObjectIDWrapper, OnchainDecimal, PythPriceIdentifier, TypeName},
    utils::{self, ptb::PTBHelper},
};
use db::models;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use rust_decimal::{prelude::*, Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{path::Path, str::FromStr, sync::Arc};
use sui_sdk::rpc_types::{Coin, SuiData, SuiMoveValue, SuiObjectDataOptions};
use sui_sdk::SuiClient;
use sui_types::base_types::{ObjectID, SuiAddress};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Obligation {
    pub id: ObjectIDWrapper,
    pub lending_market_id: ObjectID,
    pub deposits: Vec<Deposit>,
    pub borrows: Vec<Borrow>,
    pub weighted_borrowed_value_usd: OnchainDecimal,
    pub unhealthy_borrow_value_usd: OnchainDecimal,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Deposit {
    pub coin_type: crate::types::TypeName,
    #[serde_as(as = "DisplayFromStr")]
    pub deposited_ctoken_amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_array_index: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Borrow {
    pub coin_type: crate::types::TypeName,
    pub borrowed_amount: OnchainDecimal,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_array_index: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuilendMarket {
    pub id: ObjectIDWrapper,
    pub reserves: Vec<SuilendReserve>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuilendReserve {
    #[serde_as(as = "DisplayFromStr")]
    pub array_index: u64,
    pub coin_type: TypeName,
    pub config: SuilendReserveConfigCell,
    #[serde_as(as = "DisplayFromStr")]
    pub ctoken_supply: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub available_amount: u64,
    pub borrowed_amount: OnchainDecimal,
    pub unclaimed_spread_fees: OnchainDecimal,
    pub price_identifier: PythPriceIdentifier,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuilendReserveConfigCell {
    pub element: SuilendReserveConfig,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuilendReserveConfig {
    pub open_ltv_pct: u8,
    pub close_ltv_pct: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub borrow_weight_bps: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub liquidation_bonus_bps: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub protocol_liquidation_fee_bps: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationOwnerCap {
    pub id: ObjectIDWrapper,
    pub obligation_id: ObjectID,
}

pub struct SuilendService {
    platform: String,
    config: Arc<SuilendConfig>,
    client: Arc<SuiClient>,
    db_lending_service: Arc<db_service::lending::LendingService>,
    ptb_helper: Arc<PTBHelper>,
}

impl SuilendService {
    pub fn new(
        config: Arc<SuilendConfig>,
        client: Arc<SuiClient>,
        db_lending_service: Arc<db_service::lending::LendingService>,
        ptb_helper: Arc<PTBHelper>,
    ) -> Self {
        SuilendService {
            platform: constant::SUILEND_LENDING.to_string(),
            config,
            client,
            db_lending_service,
            ptb_helper,
        }
    }
}

#[async_trait]
impl LendingService for SuilendService {
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
        // if not provided, obligation id is fetched from onchain data
        let obligation_id = match obligation_id {
            Some(id) => id,
            None => self.find_obligation_id_from_address(&borrower).await?,
        };

        let obligation = self.fetch_obligation_by_id(&obligation_id).await?;

        let user_deposits = obligation
            .deposits
            .into_iter()
            .map(|deposit| crate::types::UserDeposit {
                platform: self.platform.clone(),
                borrower: borrower.clone(),
                obligation_id: Some(obligation_id.to_string()),
                coin_type: utils::format_type_name(&deposit.coin_type.name.clone(), true),
                amount: deposit.deposited_ctoken_amount.to_string(),
            })
            .collect::<Vec<_>>();

        let user_borrows = obligation
            .borrows
            .into_iter()
            .map(|borrow| crate::types::UserBorrow {
                platform: self.platform.clone(),
                borrower: borrower.clone(),
                obligation_id: Some(obligation_id.to_string()),
                coin_type: utils::format_type_name(&borrow.coin_type.name.clone(), true),
                amount: borrow.borrowed_amount.value.to_string(),
                debt_borrow_index: None, // This field is not available in Suilend
            })
            .collect::<Vec<_>>();

        Ok((user_deposits, user_borrows))
    }

    async fn fetch_user_deposit(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        _asset_id: Option<u8>,
    ) -> Result<crate::types::UserDeposit> {
        let coin_type = coin_type
            .as_deref()
            .map(|c| utils::format_type_name(c, true))
            .ok_or_else(|| {
                anyhow!(
                    "Coin type is required to fetch user deposit for borrower: {}",
                    borrower
                )
            })?;

        let (user_deposits, _user_borrows) = self
            .fetch_borrower_portfolio(borrower.clone(), obligation_id)
            .await?;

        let user_deposit = user_deposits
            .into_iter()
            .find(|deposit| deposit.coin_type == coin_type)
            .ok_or_else(|| anyhow!("Deposit not found for borrower: {}", borrower))?;

        Ok(user_deposit)
    }

    async fn fetch_user_borrow(
        &self,
        borrower: String,
        obligation_id: Option<String>,
        coin_type: Option<String>,
        _asset_id: Option<u8>,
    ) -> Result<crate::types::UserBorrow> {
        let coin_type = coin_type
            .as_deref()
            .map(|c| utils::format_type_name(c, true))
            .ok_or_else(|| {
                anyhow!(
                    "Coin type is required to fetch user borrow for borrower: {}",
                    borrower
                )
            })?;

        let (_user_deposits, user_borrows) = self
            .fetch_borrower_portfolio(borrower.clone(), obligation_id)
            .await?;

        let user_borrow = user_borrows
            .into_iter()
            .find(|borrow| borrow.coin_type == coin_type)
            .ok_or_else(|| anyhow!("Borrow not found for borrower: {}", borrower))?;

        Ok(user_borrow)
    }

    async fn find_obligation_id_from_address(&self, borrower: &str) -> Result<String> {
        // find in DB first
        let (cached_obligation_id, cached_borrower) = match self
            .db_lending_service
            .find_borrower_by_platform_and_address(&self.platform, borrower)
        {
            Ok(borrower) => {
                if borrower.status != constant::READY_STATUS {
                    (None, Some(borrower))
                } else {
                    (borrower.obligation_id.clone(), Some(borrower))
                }
            }
            Err(e) => (None, None),
        };

        if let Some(obligation_id) = cached_obligation_id {
            return Ok(obligation_id);
        }

        // fetch from on-chain data
        let borrower_address = SuiAddress::from_str(borrower)
            .map_err(|e| anyhow!("Invalid borrower address: {}", e))?;

        let obligation_owner_cap_obj = self
            .ptb_helper
            .find_owned_objects_given_owner_address_and_type(
                borrower_address,
                &self.config.obligation_owner_cap_object_type,
                true,
            )
            .await?;
        if obligation_owner_cap_obj.is_empty() {
            return Err(anyhow!(
                "No obligation owner cap found for borrower: {}",
                borrower
            ));
        }

        let obligation_owner_cap_obj = obligation_owner_cap_obj
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("No obligation owner cap found for borrower: {}", borrower))?;

        let obj_fields = obligation_owner_cap_obj
            .content
            .ok_or_else(|| anyhow!("Missing object content"))?
            .try_into_move()
            .ok_or_else(|| anyhow!("Invalid move object"))?
            .fields;

        let obligation_owner_cap: ObligationOwnerCap =
            serde_json::from_value(obj_fields.to_json_value())
                .map_err(|e| anyhow!("Failed to deserialize obligation owner cap: {}", e))?;
        let obligation_id = obligation_owner_cap.obligation_id.to_string();

        // save obligation ID to DB
        if let Some(cached_borrower) = cached_borrower {
            let borrower = crate::types::Borrower {
                platform: self.platform.clone(),
                borrower: cached_borrower.borrower.clone(),
                obligation_id: Some(obligation_id.clone()),
                status: cached_borrower.status,
            };

            if let Err(e) = self.db_lending_service.save_borrower_to_db(borrower) {
                error!("Failed to save borrower to DB: {}", e);
                return Err(e);
            }
        }

        Ok(obligation_id)
    }
}

impl SuilendService {
    async fn fetch_obligation_by_id(&self, obligation_id: &str) -> Result<Obligation> {
        let obligation_id = ObjectID::from_str(obligation_id)
            .map_err(|e| anyhow!("Invalid obligation ID: {}", e))?;

        let obligation_data_resp = self
            .client
            .read_api()
            .get_object_with_options(obligation_id, SuiObjectDataOptions::full_content())
            .await?;

        let obligation_data = obligation_data_resp.data.ok_or_else(|| {
            anyhow!(
                "Failed to get object data for obligation ID: {}",
                obligation_id
            )
        })?;

        if let Some(display_resp) = &obligation_data.display {
            info!(
                "Obligation ID: {}, Display: {:?}",
                obligation_id, display_resp
            );
        } else {
            warn!("No display data for obligation ID: {}", obligation_id);
        }

        let obligation_fields = obligation_data
            .content
            .ok_or_else(|| anyhow!("Missing object content"))?
            .try_into_move()
            .ok_or_else(|| anyhow!("Invalid move object"))?
            .fields;

        let obligation: Obligation = serde_json::from_value(obligation_fields.to_json_value())
            .map_err(|e| anyhow!("Failed to deserialize obligation fields: {}", e))?;

        Ok(obligation)
    }
}
