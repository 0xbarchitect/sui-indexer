pub mod navi;
pub mod scallop;
pub mod suilend;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositEvent {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub asset_id: Option<u8>,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawEvent {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub asset_id: Option<u8>,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowEvent {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub asset_id: Option<u8>,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepayEvent {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub asset_id: Option<u8>,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidateEvent {
    pub platform: String,
    pub borrower: String,
    pub liquidator: String,
    pub debt_coin: String,
    pub debt_asset_id: Option<u8>,
    pub debt_amount: String,
    pub collateral_coin: String,
    pub collateral_asset_id: Option<u8>,
    pub collateral_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUpdatedEvent {
    pub platform: String,
    pub coin_type: String,
    pub asset_id: Option<u8>,
    pub borrow_index: Option<String>,
    pub supply_index: Option<String>,
}
