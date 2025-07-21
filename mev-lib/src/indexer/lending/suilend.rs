use crate::{
    config::SuilendConfig,
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{self, db_service, lending},
    types::{Borrower, FixedPoint32, TypeName},
    utils,
};
use db::repositories::{
    CoinRepository, PoolRepository, UserBorrowRepository, UserDepositRepository,
};
use db::{
    models::{
        user_borrow::{self, NewUserBorrow, UpdateUserBorrow, UserBorrow},
        user_deposit::{NewUserDeposit, UpdateUserDeposit, UserDeposit},
    },
    repositories::user_deposit,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bcs;
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::str::FromStr;
use std::sync::Arc;
use std::{
    fmt::{Debug, Display},
    vec,
};
use sui_sdk::rpc_types::{SuiData, SuiMoveValue, SuiObjectDataOptions};
use sui_sdk::SuiClient;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    event::Event,
};
use tokio::{sync::mpsc, time::Instant};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Deserialize, Serialize)]
struct DepositEvent {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    ctoken_amount: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct DepositEventJson {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    ctoken_amount: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct WithdrawEvent {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    ctoken_amount: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct WithdrawEventJson {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    ctoken_amount: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct BorrowEvent {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    liquidity_amount: u64,
    origination_fee_amount: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct BorrowEventJson {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    liquidity_amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    origination_fee_amount: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct RepayEvent {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    liquidity_amount: u64,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct RepayEventJson {
    lending_market_id: SuiAddress,
    coin_type: TypeName,
    reserve_id: SuiAddress,
    obligation_id: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    liquidity_amount: u64,
}

pub struct SuiLend {
    platform: String,
    client: Arc<SuiClient>,
    config: Arc<SuilendConfig>,
    service: Arc<dyn lending::LendingService + Send + Sync>,
    db_service: Arc<db_service::lending::LendingService>,
}

impl SuiLend {
    pub fn new(
        client: Arc<SuiClient>,
        config: Arc<SuilendConfig>,
        service: Arc<dyn lending::LendingService + Send + Sync>,
        db_service: Arc<db_service::lending::LendingService>,
    ) -> Self {
        SuiLend {
            platform: constant::SUILEND_LENDING.to_string(),
            client,
            config,
            service,
            db_service,
        }
    }
}

impl Display for SuiLend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SuiLendEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for SuiLend {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::SUILEND_DEPOSIT_EVENT => {
                let event: DepositEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize deposit event: {}", e))?;

                let event = DepositEvent {
                    lending_market_id: event.lending_market_id,
                    coin_type: event.coin_type,
                    reserve_id: event.reserve_id,
                    obligation_id: event.obligation_id,
                    ctoken_amount: event.ctoken_amount,
                };

                self.process_deposit(&event, sender).await?;
            }
            constant::SUILEND_WITHDRAW_EVENT => {
                let event: WithdrawEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize withdraw event: {}", e))?;

                let event = WithdrawEvent {
                    lending_market_id: event.lending_market_id,
                    coin_type: event.coin_type,
                    reserve_id: event.reserve_id,
                    obligation_id: event.obligation_id,
                    ctoken_amount: event.ctoken_amount,
                };

                self.process_withdraw(&event, sender).await?;
            }
            constant::SUILEND_BORROW_EVENT => {
                let event: BorrowEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize borrow event: {}", e))?;

                let event = BorrowEvent {
                    lending_market_id: event.lending_market_id,
                    coin_type: event.coin_type,
                    reserve_id: event.reserve_id,
                    obligation_id: event.obligation_id,
                    liquidity_amount: event.liquidity_amount,
                    origination_fee_amount: event.origination_fee_amount,
                };

                self.process_borrow(&event, sender).await?;
            }
            constant::SUILEND_REPAY_EVENT => {
                let event: RepayEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize repay event: {}", e))?;

                let event = RepayEvent {
                    lending_market_id: event.lending_market_id,
                    coin_type: event.coin_type,
                    reserve_id: event.reserve_id,
                    obligation_id: event.obligation_id,
                    liquidity_amount: event.liquidity_amount,
                };

                self.process_repay(&event, sender).await?;
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
            constant::SUILEND_DEPOSIT_EVENT => {
                let deposit_event: DepositEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize deposit event: {}", e))?;

                self.process_deposit(&deposit_event, sender).await
            }
            constant::SUILEND_WITHDRAW_EVENT => {
                let withdraw_event: WithdrawEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize withdraw event: {}", e))?;

                self.process_withdraw(&withdraw_event, sender).await
            }
            constant::SUILEND_BORROW_EVENT => {
                let borrow_event: BorrowEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize borrow event: {}", e))?;

                self.process_borrow(&borrow_event, sender).await
            }
            constant::SUILEND_REPAY_EVENT => {
                let repay_event: RepayEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize repay event: {}", e))?;

                self.process_repay(&repay_event, sender).await
            }

            _ => {
                return Err(anyhow!("Unknown event type: {}", event_type));
            }
        }
    }

    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        // The user address is used as the event ID
        // because all the lending events: deposit, withdraw, borrow, and repay
        // are associated with the user address.
        // In a checkpoint processing scenario, we will select the latest event
        // for each user address to process, ignoring all the previous events.

        match event_type {
            constant::SUILEND_BORROW_EVENT
            | constant::SUILEND_REPAY_EVENT
            | constant::SUILEND_DEPOSIT_EVENT
            | constant::SUILEND_WITHDRAW_EVENT => {
                let sender = event.sender.to_string();
                Ok(format!("{}_{}_{}", &self.platform, &sender, event_type))
            }

            _ => Err(anyhow!("Unknown event type: {}", event_type)),
        }
    }
}

impl SuiLend {
    async fn process_deposit(&self, event: &DepositEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, &event.obligation_id.to_string())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user deposit", sender);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation_id.to_string())
                    .await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(
                sender.to_string(),
                Some(event.obligation_id.to_string()),
                Some(event.coin_type.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingDeposit(
            indexer::lending::DepositEvent {
                platform: self.platform.clone(),
                borrower: sender.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: None,
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_withdraw(&self, event: &WithdrawEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, &event.obligation_id.to_string())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user withdraw", sender);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation_id.to_string())
                    .await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(
                sender.to_string(),
                Some(event.obligation_id.to_string()),
                Some(event.coin_type.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingWithdraw(
            indexer::lending::WithdrawEvent {
                platform: self.platform.clone(),
                borrower: sender.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: None,
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_borrow(&self, event: &BorrowEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, &event.obligation_id.to_string())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user borrow", sender);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation_id.to_string())
                    .await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(
                sender.to_string(),
                Some(event.obligation_id.to_string()),
                Some(event.coin_type.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingBorrow(indexer::lending::BorrowEvent {
            platform: self.platform.clone(),
            borrower: sender.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: None,
            amount: user_borrow.amount,
        }))
    }

    async fn process_repay(&self, event: &RepayEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, &event.obligation_id.to_string())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user repay", sender);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation_id.to_string())
                    .await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(
                sender.to_string(),
                Some(event.obligation_id.to_string()),
                Some(event.coin_type.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingRepay(indexer::lending::RepayEvent {
            platform: self.platform.clone(),
            borrower: sender.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: None,
            amount: user_borrow.amount,
        }))
    }

    // helper functions
    async fn create_new_borrower(
        &self,
        address: String,
        obligation_id: String,
    ) -> Result<crate::types::Borrower> {
        let borrower = crate::types::Borrower {
            platform: self.platform.clone(),
            borrower: address.clone(),
            obligation_id: Some(obligation_id.clone()),
            status: constant::PENDING_STATUS,
        };
        self.db_service.save_borrower_to_db(borrower.clone())?;

        Ok(borrower)
    }

    async fn is_owner_obligation_id(&self, sender: &str, obligation_id: &str) -> Result<()> {
        let owner_obligation_id = self.service.find_obligation_id_from_address(sender).await?;
        info!(
            "Owner obligation ID for sender {}: {}",
            sender, owner_obligation_id
        );

        if owner_obligation_id != obligation_id {
            return Err(anyhow!(
                "Obligation ID mismatch for sender {}: expected {}, got {}",
                sender,
                owner_obligation_id,
                obligation_id
            ));
        }

        Ok(())
    }
}
