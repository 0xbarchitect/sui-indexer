use crate::models::pool_tick::{NewPoolTick, PoolTick, UpdatePoolTick};
use crate::repositories::PoolTickRepository;
use crate::DbPool;

use diesel::prelude::*;

pub struct PoolTickRepositoryImpl {
    db_pool: DbPool,
}

impl PoolTickRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        PoolTickRepositoryImpl { db_pool }
    }
}

impl PoolTickRepository for PoolTickRepositoryImpl {
    fn create(&self, pool_tick: &NewPoolTick) -> QueryResult<PoolTick> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(pool_ticks)
            .values(pool_tick)
            .get_result(&mut conn)
    }

    fn update(&self, pool_tick_id: i32, pool_tick: &UpdatePoolTick) -> QueryResult<PoolTick> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(pool_ticks.find(pool_tick_id))
            .set(pool_tick)
            .get_result(&mut conn)
    }

    fn delete(&self, pool_tick_id: i32) -> QueryResult<bool> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(pool_ticks.find(pool_tick_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, pool_tick_id: i32) -> QueryResult<PoolTick> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pool_ticks.find(pool_tick_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<PoolTick>> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pool_ticks.load(&mut conn)
    }

    fn find_by_address_and_tick_index(
        &self,
        pool_address: &str,
        pool_tick_index: i32,
    ) -> QueryResult<PoolTick> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        pool_ticks
            .filter(address.eq(pool_address).and(tick_index.eq(pool_tick_index)))
            .limit(1)
            .get_result(&mut conn)
    }

    fn find_by_address(&self, pool_address: &str) -> QueryResult<Vec<PoolTick>> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pool_ticks.filter(address.eq(pool_address)).load(&mut conn)
    }

    fn find_lower_tick_for_address(
        &self,
        address_str: &str,
        tick_index_val: i32,
    ) -> QueryResult<Option<PoolTick>> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pool_ticks
            .filter(address.eq(address_str).and(tick_index.lt(tick_index_val)))
            .order(tick_index.desc())
            .first::<PoolTick>(&mut conn)
            .optional()
    }

    fn find_higher_tick_for_address(
        &self,
        address_str: &str,
        tick_index_val: i32,
    ) -> QueryResult<Option<PoolTick>> {
        use crate::schema::pool_ticks::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pool_ticks
            .filter(address.eq(address_str).and(tick_index.gt(tick_index_val)))
            .order(tick_index.asc())
            .first::<PoolTick>(&mut conn)
            .optional()
    }
}
