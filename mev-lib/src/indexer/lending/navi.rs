use crate::{
    config::NaviConfig,
    constant,
    indexer::{self, EventProcessor, OnchainEvent},
    service::{db_service, lending},
    types::Borrower,
    types::U256,
};
use db::models::{
    coin::{Coin, NewCoin, UpdateCoin},
    pool::{NewPool, Pool, UpdatePool},
    user_borrow::{NewUserBorrow, UpdateUserBorrow, UserBorrow},
    user_deposit::{NewUserDeposit, UpdateUserDeposit, UserDeposit},
};
use db::repositories::{
    CoinRepository, PoolRepository, UserBorrowRepository, UserDepositRepository,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bcs;
use core::borrow;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use serde_with::{serde_as, DisplayFromStr};
use std::{
    f32::consts::E,
    fmt::{Debug, Display},
    str::FromStr,
    sync::Arc,
};
use sui_json_rpc_types::{SuiObjectDataOptions, SuiParsedData};
use sui_sdk::SuiClient;
use sui_types::{
    base_types::{ObjectID, SequenceNumber, SuiAddress},
    event::Event,
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
    pub reserve: u8,
    pub sender: SuiAddress,
    pub amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DepositEventJson {
    pub reserve: u8,
    pub sender: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WithdrawEvent {
    pub reserve: u8,
    pub sender: SuiAddress,
    pub to: SuiAddress,
    pub amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct WithdrawEventJson {
    pub reserve: u8,
    pub sender: SuiAddress,
    pub to: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BorrowEvent {
    pub reserve: u8,
    pub sender: SuiAddress,
    pub amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct BorrowEventJson {
    pub reserve: u8,
    pub sender: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RepayEvent {
    pub reserve: u8,
    pub sender: SuiAddress,
    pub amount: u64,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RepayEventJson {
    pub reserve: u8,
    pub sender: SuiAddress,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct StateUpdatedEvent {
    pub user: SuiAddress,
    pub asset: u8,
    pub user_supply_balance: U256,
    pub user_borrow_balance: U256,
    pub new_supply_index: U256,
    pub new_borrow_index: U256,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
struct StateUpdatedEventJson {
    pub user: SuiAddress,
    pub asset: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub user_supply_balance: U256,
    #[serde_as(as = "DisplayFromStr")]
    pub user_borrow_balance: U256,
    #[serde_as(as = "DisplayFromStr")]
    pub new_supply_index: U256,
    #[serde_as(as = "DisplayFromStr")]
    pub new_borrow_index: U256,
}

pub struct Navi {
    platform: String,
    client: Arc<SuiClient>,
    config: Arc<NaviConfig>,
    service: Arc<dyn lending::LendingService + Send + Sync>,
    db_service: Arc<db_service::lending::LendingService>,
}

impl Navi {
    pub fn new(
        client: Arc<SuiClient>,
        config: Arc<NaviConfig>,
        service: Arc<dyn lending::LendingService + Send + Sync>,
        db_service: Arc<db_service::lending::LendingService>,
    ) -> Self {
        Navi {
            platform: constant::NAVI_LENDING.to_string(),
            client,
            config,
            service,
            db_service,
        }
    }
}

impl Display for Navi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NaviEventProcessor")
    }
}

#[async_trait]
impl EventProcessor for Navi {
    async fn process_tx_event(
        &self,
        event_type: &str,
        sender: &str,
        data: Value,
        tx_digest: &str,
    ) -> Result<()> {
        match event_type {
            constant::NAVI_DEPOSIT_EVENT => {
                let event: DepositEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize deposit event: {}", e))?;

                let event = DepositEvent {
                    reserve: event.reserve,
                    amount: event.amount,
                    sender: event.sender,
                };

                self.process_deposit(&event).await?;
            }
            constant::NAVI_WITHDRAW_EVENT => {
                let event: WithdrawEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize withdraw event: {}", e))?;

                let event = WithdrawEvent {
                    reserve: event.reserve,
                    amount: event.amount,
                    sender: event.sender,
                    to: event.to,
                };

                self.process_withdraw(&event).await?;
            }
            constant::NAVI_BORROW_EVENT => {
                let event: BorrowEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize borrow event: {}", e))?;

                let event = BorrowEvent {
                    reserve: event.reserve,
                    amount: event.amount,
                    sender: event.sender,
                };

                self.process_borrow(&event).await?;
            }
            constant::NAVI_REPAY_EVENT => {
                let event: RepayEventJson = serde_json::from_value(data)
                    .map_err(|e| anyhow!("Failed to deserialize repay event: {}", e))?;

                let event = RepayEvent {
                    reserve: event.reserve,
                    amount: event.amount,
                    sender: event.sender,
                };

                self.process_repay(&event).await?;
            }
            _ => return Err(anyhow!("Unsupported event type: {}", event_type)),
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
            constant::NAVI_DEPOSIT_EVENT => {
                let event: DepositEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize deposit event: {}", e))?;

                self.process_deposit(&event).await
            }
            constant::NAVI_WITHDRAW_EVENT => {
                let event: WithdrawEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize withdraw event: {}", e))?;

                self.process_withdraw(&event).await
            }
            constant::NAVI_BORROW_EVENT => {
                let event: BorrowEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize borrow event: {}", e))?;

                self.process_borrow(&event).await
            }
            constant::NAVI_REPAY_EVENT => {
                let event: RepayEvent = bcs::from_bytes(&event.contents)
                    .map_err(|e| anyhow!("Failed to deserialize repay event: {}", e))?;

                self.process_repay(&event).await
            }

            _ => return Err(anyhow!("Unsupported event type: {}", event_type)),
        }
    }

    fn get_event_id(&self, event_type: &str, event: &Event) -> Result<String> {
        match event_type {
            constant::NAVI_DEPOSIT_EVENT
            | constant::NAVI_WITHDRAW_EVENT
            | constant::NAVI_BORROW_EVENT
            | constant::NAVI_REPAY_EVENT => Ok(format!(
                "{}_{}_{}",
                &self.platform,
                &event.sender.to_string(),
                event_type
            )),

            _ => Err(anyhow!("Unsupported event type: {}", event_type)),
        }
    }
}

impl Navi {
    async fn process_deposit(&self, event: &DepositEvent) -> Result<OnchainEvent> {
        info!("Processing Navi deposit event: {:?}", event);

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, &event.sender.to_string())
        {
            Ok(borrower) => {
                // if borrower exists and has been fully initialized, update user_deposit
                info!("Borrower {} exists, updating user deposit", event.sender);
            }
            Err(e) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(&event.sender.to_string()).await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(event.sender.to_string(), None, None, Some(event.reserve))
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingDeposit(
            indexer::lending::DepositEvent {
                platform: self.platform.clone(),
                borrower: event.sender.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: Some(event.reserve),
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_withdraw(&self, event: &WithdrawEvent) -> Result<OnchainEvent> {
        info!("Processing Navi withdraw event: {:?}", event);

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, &event.sender.to_string())
        {
            Ok(borrower) => {
                // if borrower exists and has been fully initialized, update user_deposit
                info!("Borrower {} exists, updating user deposit", event.sender);
            }
            Err(e) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(&event.sender.to_string()).await?;
            }
        }

        let user_deposit = self
            .service
            .fetch_user_deposit(event.sender.to_string(), None, None, Some(event.reserve))
            .await?;

        self.db_service
            .save_user_deposit_to_db(user_deposit.clone())
            .await?;

        Ok(OnchainEvent::LendingWithdraw(
            indexer::lending::WithdrawEvent {
                platform: self.platform.clone(),
                borrower: event.sender.to_string(),
                coin_type: user_deposit.coin_type,
                asset_id: Some(event.reserve),
                amount: user_deposit.amount,
            },
        ))
    }

    async fn process_borrow(&self, event: &BorrowEvent) -> Result<OnchainEvent> {
        info!("Processing Navi borrow event: {:?}", event);

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, &event.sender.to_string())
        {
            Ok(borrower) => {
                // if borrower exists and has been fully initialized, update user_deposit
                info!("Borrower {} exists, updating user borrow", event.sender);
            }
            Err(e) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(&event.sender.to_string()).await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(event.sender.to_string(), None, None, Some(event.reserve))
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingBorrow(indexer::lending::BorrowEvent {
            platform: self.platform.clone(),
            borrower: event.sender.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: Some(event.reserve),
            amount: user_borrow.amount,
        }))
    }

    async fn process_repay(&self, event: &RepayEvent) -> Result<OnchainEvent> {
        info!("Processing Navi repay event: {:?}", event);

        match self
            .db_service
            .find_borrower_by_platform_and_address(&self.platform, &event.sender.to_string())
        {
            Ok(borrower) => {
                // if borrower exists and has been fully initialized, update user_deposit
                info!("Borrower {} exists, updating user repay", event.sender);
            }
            Err(e) => {
                // If borrower does not exist, create a new borrower entry
                self.create_new_borrower(&event.sender.to_string()).await?;
            }
        }

        let user_borrow = self
            .service
            .fetch_user_borrow(event.sender.to_string(), None, None, Some(event.reserve))
            .await?;

        self.db_service
            .save_user_borrow_to_db(user_borrow.clone())
            .await?;

        Ok(OnchainEvent::LendingRepay(indexer::lending::RepayEvent {
            platform: self.platform.clone(),
            borrower: event.sender.to_string(),
            coin_type: user_borrow.coin_type,
            asset_id: Some(event.reserve),
            amount: user_borrow.amount,
        }))
    }

    async fn create_new_borrower(&self, address: &str) -> Result<crate::types::Borrower> {
        let borrower = crate::types::Borrower {
            platform: self.platform.clone(),
            borrower: address.to_string(),
            obligation_id: None,
            status: constant::PENDING_STATUS,
        };
        self.db_service.save_borrower_to_db(borrower.clone())?;

        Ok(borrower)
    }
}
