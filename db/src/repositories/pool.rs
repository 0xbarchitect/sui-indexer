use crate::models::pool::{NewPool, Pool, UpdatePool};
use crate::repositories::PoolRepository;
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

pub struct PoolRepositoryImpl {
    db_pool: DbPool,
}

impl PoolRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        PoolRepositoryImpl { db_pool }
    }
}

impl PoolRepository for PoolRepositoryImpl {
    fn create(&self, pool: &NewPool) -> QueryResult<Pool> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(pools)
            .values(pool)
            .get_result(&mut conn)
    }

    fn update(&self, pool_id: i32, pool: &UpdatePool) -> QueryResult<Pool> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(pools.find(pool_id))
            .set(pool)
            .get_result(&mut conn)
    }

    fn delete(&self, pool_id: i32) -> QueryResult<bool> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(pools.find(pool_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, pool_id: i32) -> QueryResult<Pool> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pools.find(pool_id).get_result(&mut conn)
    }

    fn find_by_address(&self, pool_address: &str) -> QueryResult<Pool> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pools
            .filter(address.eq(pool_address))
            .limit(1)
            .get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<Pool>> {
        use crate::schema::pools::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        pools.load(&mut conn)
    }
}
