use crate::schema::coins;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = coins)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Coin {
    pub id: i32,
    pub coin_type: String,
    pub decimals: i32,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub price_pyth: Option<String>,
    pub price_supra: Option<String>,
    pub price_switchboard: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub pyth_feed_id: Option<String>,
    pub pyth_info_object_id: Option<String>,
    pub pyth_latest_updated_at: Option<NaiveDateTime>,
    pub pyth_ema_price: Option<String>,
    pub pyth_decimals: Option<i32>,
    pub navi_asset_id: Option<i32>,
    pub navi_oracle_id: Option<i32>,
    pub navi_feed_id: Option<String>,
    pub hermes_price: Option<String>,
    pub hermes_latest_updated_at: Option<NaiveDateTime>,
    pub vaa: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = coins)]
pub struct NewCoin {
    pub coin_type: String,
    pub decimals: i32,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub price_pyth: Option<String>,
    pub price_supra: Option<String>,
    pub price_switchboard: Option<String>,
    pub pyth_feed_id: Option<String>,
    pub pyth_info_object_id: Option<String>,
    pub pyth_latest_updated_at: Option<NaiveDateTime>,
    pub pyth_ema_price: Option<String>,
    pub pyth_decimals: Option<i32>,
    pub navi_asset_id: Option<i32>,
    pub navi_oracle_id: Option<i32>,
    pub navi_feed_id: Option<String>,
    pub hermes_price: Option<String>,
    pub hermes_latest_updated_at: Option<NaiveDateTime>,
    pub vaa: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = coins)]
pub struct UpdateCoin {
    pub coin_type: Option<String>,
    pub decimals: Option<i32>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub price_pyth: Option<String>,
    pub price_supra: Option<String>,
    pub price_switchboard: Option<String>,
    pub pyth_feed_id: Option<String>,
    pub pyth_info_object_id: Option<String>,
    pub pyth_latest_updated_at: Option<NaiveDateTime>,
    pub pyth_ema_price: Option<String>,
    pub pyth_decimals: Option<i32>,
    pub navi_asset_id: Option<i32>,
    pub navi_oracle_id: Option<i32>,
    pub navi_feed_id: Option<String>,
    pub hermes_price: Option<String>,
    pub hermes_latest_updated_at: Option<NaiveDateTime>,
    pub vaa: Option<String>,
}
