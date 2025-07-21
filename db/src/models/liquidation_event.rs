use crate::schema::liquidation_events;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = liquidation_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LiquidationEvent {
    pub id: i32,
    pub tx_digest: String,
    pub platform: String,
    pub borrower: Option<String>,
    pub liquidator: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = liquidation_events)]
pub struct NewLiquidationEvent {
    pub tx_digest: String,
    pub platform: String,
    pub borrower: Option<String>,
    pub liquidator: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = liquidation_events)]
pub struct UpdateLiquidationEvent {
    pub tx_digest: Option<String>,
    pub platform: Option<String>,
    pub borrower: Option<String>,
    pub liquidator: Option<String>,
}
