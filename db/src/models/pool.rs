use crate::schema::pools;
use chrono::NaiveDateTime;

use diesel::prelude::*;
use diesel::sql_types::{Array, Float8, Integer, Text};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = pools)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Pool {
    pub id: i32,
    pub exchange: String,
    pub address: String,
    pub liquidity: Option<String>,
    pub current_sqrt_price: Option<String>,
    pub tick_spacing: Option<i32>,
    pub fee_rate: Option<i32>,
    pub is_pause: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub coins: String,
    pub coin_amounts: Option<String>,
    pub weights: Option<String>,
    pub fees_swap_in: Option<String>,
    pub fees_swap_out: Option<String>,
    pub current_tick_index: Option<i32>,
    pub pool_type: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = pools)]
pub struct NewPool {
    pub exchange: String,
    pub address: String,
    pub liquidity: Option<String>,
    pub current_sqrt_price: Option<String>,
    pub tick_spacing: Option<i32>,
    pub fee_rate: Option<i32>,
    pub is_pause: Option<bool>,
    pub coins: String,
    pub coin_amounts: Option<String>,
    pub weights: Option<String>,
    pub fees_swap_in: Option<String>,
    pub fees_swap_out: Option<String>,
    pub current_tick_index: Option<i32>,
    pub pool_type: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = pools)]
pub struct UpdatePool {
    pub exchange: Option<String>,
    pub address: Option<String>,
    pub liquidity: Option<String>,
    pub current_sqrt_price: Option<String>,
    pub tick_spacing: Option<i32>,
    pub fee_rate: Option<i32>,
    pub is_pause: Option<bool>,
    pub coins: Option<String>,
    pub coin_amounts: Option<String>,
    pub weights: Option<String>,
    pub fees_swap_in: Option<String>,
    pub fees_swap_out: Option<String>,
    pub current_tick_index: Option<i32>,
    pub pool_type: Option<String>,
}
