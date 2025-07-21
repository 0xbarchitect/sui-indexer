use crate::{
    config::ScallopConfig,
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{db_service, lending},
    types::{Borrower, FixedPoint32, FixedPoint32Json, TypeName},
    utils,
};
use db::models::{
    user_borrow::{NewUserBorrow, UpdateUserBorrow, UserBorrow},
    user_deposit::{self, NewUserDeposit, UpdateUserDeposit, UserDeposit},
};
use db::repositories::{
    CoinRepository, PoolRepository, UserBorrowRepository, UserDepositRepository,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bcs;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use sui_sdk::{
    rpc_types::{
        SuiData, SuiMoveValue, SuiObjectData, SuiObjectDataFilter, SuiObjectDataOptions,
        SuiObjectResponseQuery,
    },
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, ObjectType, SequenceNumber, SuiAddress},
    event::{self, Event},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Command, ObjectArg, TransactionData, TransactionKind},
    Identifier,
};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DepositEvent {
    pub provider: SuiAddress,
    pub obligation: ObjectID,
    pub deposit_asset: TypeName,
    pub deposit_amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DepositEventJson {
    pub provider: SuiAddress,
    pub obligation: ObjectID,
    pub deposit_asset: TypeName,
    #[serde_as(as = "DisplayFromStr")]
    pub deposit_amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WithdrawEvent {
    pub taker: SuiAddress,
    pub obligation: ObjectID,
    pub withdraw_asset: TypeName,
    pub withdraw_amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct WithdrawEventJson {
    pub taker: SuiAddress,
    pub obligation: ObjectID,
    pub withdraw_asset: TypeName,
    #[serde_as(as = "DisplayFromStr")]
    pub withdraw_amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BorrowEventV3 {
    pub borrower: SuiAddress,
    pub obligation: ObjectID,
    pub asset: TypeName,
    pub amount: u64,
    pub borrow_fee: u64,
    pub borrow_fee_discount: u64,
    pub borrow_referral_fee: u64,
    pub time: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct BorrowEventV3Json {
    pub borrower: SuiAddress,
    pub obligation: ObjectID,
    pub asset: TypeName,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub borrow_fee: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub borrow_fee_discount: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub borrow_referral_fee: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub time: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RepayEvent {
    pub repayer: SuiAddress,
    pub obligation: ObjectID,
    pub asset: TypeName,
    pub amount: u64,
    pub time: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RepayEventJson {
    pub repayer: SuiAddress,
    pub obligation: ObjectID,
    pub asset: TypeName,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub time: u64,
}

pub struct Scallop {
    platform: String,
    client: Arc<SuiClient>,
    config: Arc<ScallopConfig>,
    service: Arc<dyn lending::LendingService + Send + Sync>,
    db_service: Arc<db_service::lending::LendingService>,
}

impl Scallop {
    pub fn new(
        client: Arc<SuiClient>,
        config: Arc<ScallopConfig>,
        service: Arc<dyn lending::LendingService + Send + Sync>,
        db_service: Arc<db_service::lending::LendingService>,
    ) -> Self {
        Self {
            platform: constant::SCALLOP_LENDING.to_string(),
            client,
            config,
            service,
            db_service,
        }
    }
}

impl Display for Scallop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScallopEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Scallop {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::SCALLOP_DEPOSIT_EVENT => {
                let event: DepositEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize deposit event: {}", e))?;

                let event = DepositEvent {
                    provider: event.provider,
                    obligation: event.obligation,
                    deposit_asset: event.deposit_asset,
                    deposit_amount: event.deposit_amount,
                };

                self.process_deposit(&event, sender).await?;
            }
            constant::SCALLOP_WITHDRAW_EVENT => {
                let event: WithdrawEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize withdraw event: {}", e))?;

                let event = WithdrawEvent {
                    taker: event.taker,
                    obligation: event.obligation,
                    withdraw_asset: event.withdraw_asset,
                    withdraw_amount: event.withdraw_amount,
                };

                self.process_withdraw(&event, sender).await?;
            }
            constant::SCALLOP_BORROW_EVENT_V3 => {
                let event: BorrowEventV3Json = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize borrow event: {}", e))?;

                let event = BorrowEventV3 {
                    borrower: event.borrower,
                    obligation: event.obligation,
                    asset: event.asset,
                    amount: event.amount,
                    borrow_fee: event.borrow_fee,
                    borrow_fee_discount: event.borrow_fee_discount,
                    borrow_referral_fee: event.borrow_referral_fee,
                    time: event.time,
                };

                self.process_borrow(&event, sender).await?;
            }
            constant::SCALLOP_REPAY_EVENT => {
                let event: RepayEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize repay event: {}", e))?;

                let event = RepayEvent {
                    repayer: event.repayer,
                    obligation: event.obligation,
                    asset: event.asset,
                    amount: event.amount,
                    time: event.time,
                };

                self.process_repay(&event, sender).await?;
            }

            _ => {
                error!("Unsupported event type: {}", event_type);
                return Err(anyhow!("Unsupported event type: {}", event_type));
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
            constant::SCALLOP_DEPOSIT_EVENT => {
                let event: DepositEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode deposit event: {}", e))?;

                self.process_deposit(&event, sender).await
            }
            constant::SCALLOP_WITHDRAW_EVENT => {
                let event: WithdrawEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode withdraw event: {}", e))?;

                self.process_withdraw(&event, sender).await
            }
            constant::SCALLOP_BORROW_EVENT_V3 => {
                let event: BorrowEventV3 = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode borrow event: {}", e))?;

                self.process_borrow(&event, sender).await
            }

            constant::SCALLOP_REPAY_EVENT => {
                let event: RepayEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to decode repay event: {}", e))?;

                self.process_repay(&event, sender).await
            }

            _ => {
                return Err(anyhow!("Unsupported event type: {}", event_type));
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
            constant::SCALLOP_DEPOSIT_EVENT
            | constant::SCALLOP_WITHDRAW_EVENT
            | constant::SCALLOP_BORROW_EVENT
            | constant::SCALLOP_BORROW_EVENT_V2
            | constant::SCALLOP_BORROW_EVENT_V3
            | constant::SCALLOP_REPAY_EVENT => {
                let sender = event.sender.to_string();

                Ok(format!("{}_{}_{}", &self.platform, &sender, event_type))
            }

            _ => Err(anyhow!("Unsupported event type: {}", event_type)),
        }
    }
}

impl Scallop {
    async fn process_deposit(&self, event: &DepositEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, event.obligation.to_string().as_str())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user deposit", event.provider);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation.to_string())
                    .await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(
                event.provider.to_string(),
                Some(event.obligation.to_string()),
                Some(event.deposit_asset.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingDeposit(
            indexer::lending::DepositEvent {
                platform: self.platform.clone(),
                borrower: event.provider.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: None,
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_withdraw(&self, event: &WithdrawEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, event.obligation.to_string().as_str())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user withdraw", event.taker);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation.to_string())
                    .await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(
                event.taker.to_string(),
                Some(event.obligation.to_string()),
                Some(event.withdraw_asset.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingWithdraw(
            indexer::lending::WithdrawEvent {
                platform: self.platform.clone(),
                borrower: event.taker.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: None,
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_borrow(&self, event: &BorrowEventV3, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, event.obligation.to_string().as_str())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user borrow", event.borrower);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation.to_string())
                    .await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(
                event.borrower.to_string(),
                Some(event.obligation.to_string()),
                Some(event.asset.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingBorrow(indexer::lending::BorrowEvent {
            platform: self.platform.clone(),
            borrower: event.borrower.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: None,
            amount: user_borrow.amount,
        }))
    }

    async fn process_repay(&self, event: &RepayEvent, sender: &str) -> Result<OnchainEvent> {
        self.is_owner_obligation_id(sender, event.obligation.to_string().as_str())
            .await?;

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, sender)
        {
            Ok(borrower) => {
                info!("Borrower {} exists, updating user repay", event.repayer);
            }
            Err(_) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(sender.to_string(), event.obligation.to_string())
                    .await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(
                event.repayer.to_string(),
                Some(event.obligation.to_string()),
                Some(event.asset.name.clone()),
                None,
            )
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingRepay(indexer::lending::RepayEvent {
            platform: self.platform.clone(),
            borrower: event.repayer.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: None,
            amount: user_borrow.amount,
        }))
    }

    // helper functions
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
}
