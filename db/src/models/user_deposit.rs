use crate::schema::user_deposits;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::*;

use std::hash::{Hash, Hasher};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = user_deposits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserDeposit {
    pub id: i32,
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub amount: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub obligation_id: Option<String>,
}

impl PartialEq for UserDeposit {
    fn eq(&self, other: &Self) -> bool {
        self.platform == other.platform
            && self.borrower == other.borrower
            && self.coin_type == other.coin_type
    }
}

impl Eq for UserDeposit {}

impl Hash for UserDeposit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.platform.hash(state);
        self.borrower.hash(state);
        self.coin_type.hash(state);
    }
}

#[derive(Insertable)]
#[diesel(table_name = user_deposits)]
pub struct NewUserDeposit {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub amount: String,
    pub obligation_id: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = user_deposits)]
pub struct UpdateUserDeposit {
    pub platform: Option<String>,
    pub borrower: Option<String>,
    pub coin_type: Option<String>,
    pub amount: Option<String>,
    pub obligation_id: Option<String>,
}

#[derive(QueryableByName, Debug, Clone)]
pub struct UserDepositWithCoinInfo {
    #[diesel(sql_type = Text)]
    pub platform: String,
    #[diesel(sql_type = Text)]
    pub borrower: String,
    #[diesel(sql_type = Text)]
    pub coin_type: String,
    #[diesel(sql_type = Text)]
    pub amount: String,
    #[diesel(sql_type = Integer)]
    pub decimals: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub price_pyth: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub pyth_decimals: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pyth_feed_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pyth_info_object_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub vaa: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub navi_feed_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub liquidation_threshold: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub asset_id: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pool_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub borrow_index: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub supply_index: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub ctoken_supply: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub available_amount: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub borrowed_amount: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub unclaimed_spread_fees: Option<String>,
}

#[derive(Queryable, Debug)]
pub struct UserDepositDistinct {
    #[diesel(sql_type = Text)]
    pub platform: String,
    #[diesel(sql_type = Text)]
    pub borrower: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub obligation_id: Option<String>,
}
