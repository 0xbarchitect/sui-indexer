use crate::schema::liquidation_orders;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = liquidation_orders)]
pub struct LiquidationOrder {
    pub id: i32,
    pub platform: String,
    pub borrower: String,
    pub hf: f32,
    pub debt_coin: String,
    pub collateral_coin: String,
    pub amount_repay: String,
    pub source: String,
    pub tx_digest: Option<String>,
    pub checkpoint: Option<i64>,
    pub bot_address: Option<String>,
    pub finalized_at: Option<NaiveDateTime>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub status: Option<i32>,
    pub amount_usd: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = liquidation_orders)]
pub struct NewLiquidationOrder {
    pub platform: String,
    pub borrower: String,
    pub hf: f32,
    pub debt_coin: String,
    pub collateral_coin: String,
    pub amount_repay: String,
    pub source: String,
    pub tx_digest: Option<String>,
    pub checkpoint: Option<i64>,
    pub bot_address: Option<String>,
    pub finalized_at: Option<NaiveDateTime>,
    pub status: Option<i32>,
    pub amount_usd: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = liquidation_orders)]
pub struct UpdateLiquidationOrder {
    pub platform: Option<String>,
    pub borrower: Option<String>,
    pub hf: Option<f32>,
    pub debt_coin: Option<String>,
    pub collateral_coin: Option<String>,
    pub amount_repay: Option<String>,
    pub source: Option<String>,
    pub tx_digest: Option<String>,
    pub checkpoint: Option<i64>,
    pub bot_address: Option<String>,
    pub finalized_at: Option<NaiveDateTime>,
    pub status: Option<i32>,
    pub amount_usd: Option<String>,
    pub error: Option<String>,
}

#[derive(QueryableByName, Debug, Clone)]
pub struct TopLiquidationOrder {
    #[diesel(sql_type = Text)]
    pub platform: String,
    #[diesel(sql_type = Text)]
    pub borrower: String,
    #[diesel(sql_type = Float)]
    pub hf: f32,
    #[diesel(sql_type = Text)]
    pub debt_coin: String,
    #[diesel(sql_type = Text)]
    pub collateral_coin: String,
    #[diesel(sql_type = Text)]
    pub amount_repay: String,
    #[diesel(sql_type = Text)]
    pub amount_usd: String,
}
