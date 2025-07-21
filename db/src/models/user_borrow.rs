use crate::schema::user_borrows;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::*;

use std::hash::{Hash, Hasher};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = user_borrows)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserBorrow {
    pub id: i32,
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub amount: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub obligation_id: Option<String>,
    pub debt_borrow_index: Option<String>,
}

impl PartialEq for UserBorrow {
    fn eq(&self, other: &Self) -> bool {
        self.platform == other.platform
            && self.borrower == other.borrower
            && self.coin_type == other.coin_type
    }
}

impl Eq for UserBorrow {}

impl Hash for UserBorrow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.platform.hash(state);
        self.borrower.hash(state);
        self.coin_type.hash(state);
    }
}

#[derive(Insertable)]
#[diesel(table_name = user_borrows)]
pub struct NewUserBorrow {
    pub platform: String,
    pub borrower: String,
    pub coin_type: String,
    pub amount: String,
    pub obligation_id: Option<String>,
    pub debt_borrow_index: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = user_borrows)]
pub struct UpdateUserBorrow {
    pub platform: Option<String>,
    pub borrower: Option<String>,
    pub coin_type: Option<String>,
    pub amount: Option<String>,
    pub obligation_id: Option<String>,
    pub debt_borrow_index: Option<String>,
}

#[derive(QueryableByName, Debug, Clone)]
pub struct UserBorrowWithCoinInfo {
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
    #[diesel(sql_type = Nullable<Text>)]
    pub borrow_index: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub supply_index: Option<String>,
}

#[derive(Queryable, Debug)]
pub struct UserBorrowDistinct {
    #[diesel(sql_type = Text)]
    pub platform: String,
    #[diesel(sql_type = Text)]
    pub borrower: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub obligation_id: Option<String>,
}

#[derive(QueryableByName, Debug)]
pub struct UserBorrowCoin {
    #[diesel(sql_type = Text)]
    pub coin_type: String,
}
