use crate::schema::pool_ticks;
use chrono::NaiveDateTime;

use diesel::prelude::*;
use diesel::sql_types::{Array, Float8, Integer, Text};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = pool_ticks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PoolTick {
    pub id: i32,
    pub address: String,
    pub tick_index: i32,
    pub liquidity_net: Option<String>,
    pub liquidity_gross: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = pool_ticks)]
pub struct NewPoolTick {
    pub address: String,
    pub tick_index: i32,
    pub liquidity_net: Option<String>,
    pub liquidity_gross: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = pool_ticks)]
pub struct UpdatePoolTick {
    pub address: Option<String>,
    pub tick_index: Option<i32>,
    pub liquidity_net: Option<String>,
    pub liquidity_gross: Option<String>,
}
