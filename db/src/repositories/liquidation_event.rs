use crate::models::liquidation_event::{
    LiquidationEvent, NewLiquidationEvent, UpdateLiquidationEvent,
};
use crate::repositories::LiquidationEventRepository;
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

pub struct LiquidationEventRepositoryImpl {
    db_pool: DbPool,
}

impl LiquidationEventRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}

impl LiquidationEventRepository for LiquidationEventRepositoryImpl {
    fn create(&self, liquidation_event: &NewLiquidationEvent) -> QueryResult<LiquidationEvent> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(liquidation_events)
            .values(liquidation_event)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        liquidation_event_id: i32,
        liquidation_event: &UpdateLiquidationEvent,
    ) -> QueryResult<LiquidationEvent> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(liquidation_events.find(liquidation_event_id))
            .set(liquidation_event)
            .get_result(&mut conn)
    }

    fn delete(&self, liquidation_event_id: i32) -> QueryResult<bool> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let rows_deleted =
            diesel::delete(liquidation_events.find(liquidation_event_id)).execute(&mut conn)?;
        Ok(rows_deleted > 0)
    }

    fn find_by_id(&self, liquidation_event_id: i32) -> QueryResult<LiquidationEvent> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        liquidation_events
            .find(liquidation_event_id)
            .get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<LiquidationEvent>> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        liquidation_events.load(&mut conn)
    }

    fn find_by_tx_digest(&self, tx_digest_str: &str) -> QueryResult<LiquidationEvent> {
        use crate::schema::liquidation_events::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        liquidation_events
            .filter(tx_digest.eq(tx_digest_str))
            .first(&mut conn)
            .map_err(|e| e.into())
    }
}
