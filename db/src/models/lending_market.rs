use crate::schema::lending_markets;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::*;

use serde_json::Value;
use std::hash::{Hash, Hasher};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = lending_markets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LendingMarket {
    pub id: i32,
    pub platform: String,
    pub coin_type: String,
    pub ltv: Option<String>,
    pub liquidation_threshold: Option<String>,
    pub borrow_weight: Option<String>,
    pub liquidation_ratio: Option<String>,
    pub liquidation_penalty: Option<String>,
    pub liquidation_fee: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub asset_id: Option<i32>,
    pub pool_id: Option<String>,
    pub borrow_index: Option<String>,
    pub supply_index: Option<String>,
    pub flashloan_path: Option<Value>,
    pub ctoken_supply: Option<String>,
    pub available_amount: Option<String>,
    pub borrowed_amount: Option<String>,
    pub unclaimed_spread_fees: Option<String>,
    pub pyth_feed_id: Option<String>,
}

impl PartialEq for LendingMarket {
    fn eq(&self, other: &Self) -> bool {
        self.platform == other.platform && self.coin_type == other.coin_type
    }
}

impl Eq for LendingMarket {}

impl Hash for LendingMarket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.platform.hash(state);
        self.coin_type.hash(state);
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = lending_markets)]
pub struct NewLendingMarket {
    pub platform: String,
    pub coin_type: String,
    pub ltv: Option<String>,
    pub liquidation_threshold: Option<String>,
    pub borrow_weight: Option<String>,
    pub liquidation_ratio: Option<String>,
    pub liquidation_penalty: Option<String>,
    pub liquidation_fee: Option<String>,
    pub asset_id: Option<i32>,
    pub pool_id: Option<String>,
    pub borrow_index: Option<String>,
    pub supply_index: Option<String>,
    pub flashloan_path: Option<Value>,
    pub ctoken_supply: Option<String>,
    pub available_amount: Option<String>,
    pub borrowed_amount: Option<String>,
    pub unclaimed_spread_fees: Option<String>,
    pub pyth_feed_id: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = lending_markets)]
pub struct UpdateLendingMarket {
    pub platform: Option<String>,
    pub coin_type: Option<String>,
    pub ltv: Option<String>,
    pub liquidation_threshold: Option<String>,
    pub borrow_weight: Option<String>,
    pub liquidation_ratio: Option<String>,
    pub liquidation_penalty: Option<String>,
    pub liquidation_fee: Option<String>,
    pub asset_id: Option<i32>,
    pub pool_id: Option<String>,
    pub borrow_index: Option<String>,
    pub supply_index: Option<String>,
    pub flashloan_path: Option<Value>,
    pub ctoken_supply: Option<String>,
    pub available_amount: Option<String>,
    pub borrowed_amount: Option<String>,
    pub unclaimed_spread_fees: Option<String>,
    pub pyth_feed_id: Option<String>,
}

#[derive(QueryableByName, Debug)]
pub struct LendingMarketWithCoinInfo {
    #[diesel(sql_type = Text)]
    pub platform: String,
    #[diesel(sql_type = Text)]
    pub coin_type: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub ltv: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub liquidation_threshold: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub borrow_weight: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub liquidation_ratio: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub liquidation_penalty: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub liquidation_fee: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub asset_id: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pool_id: Option<String>,
    #[diesel(sql_type = Integer)]
    pub decimals: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub price_pyth: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub pyth_decimals: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pyth_info_object_id: Option<String>,
}
