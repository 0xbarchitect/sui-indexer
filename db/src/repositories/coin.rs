use crate::models::coin::{Coin, NewCoin, UpdateCoin};
use crate::repositories::CoinRepository;
use crate::DbPool;

use diesel::prelude::*;

pub struct CoinRepositoryImpl {
    db_pool: DbPool,
}

impl CoinRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        CoinRepositoryImpl { db_pool }
    }
}

impl CoinRepository for CoinRepositoryImpl {
    fn create(&self, coin: &NewCoin) -> QueryResult<Coin> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(coins)
            .values(coin)
            .get_result(&mut conn)
    }

    fn update(&self, coin_id: i32, coin: &UpdateCoin) -> QueryResult<Coin> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(coins.find(coin_id))
            .set(coin)
            .get_result(&mut conn)
    }

    fn delete(&self, coin_id: i32) -> QueryResult<bool> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(coins.find(coin_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, coin_id: i32) -> QueryResult<Coin> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        coins.find(coin_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<Coin>> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        coins.load(&mut conn)
    }

    fn find_by_coin_type(&self, coin_type_str: &str) -> QueryResult<Coin> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        coins
            .filter(coin_type.eq(coin_type_str))
            .limit(1)
            .get_result(&mut conn)
    }

    fn find_by_pyth_feed_id(&self, feed_id: &str) -> QueryResult<Vec<Coin>> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        coins.filter(pyth_feed_id.eq(feed_id)).load(&mut conn)
    }

    fn find_by_navi_asset_id(&self, asset_id: i32) -> QueryResult<Coin> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        coins
            .filter(navi_asset_id.eq(asset_id))
            .limit(1)
            .get_result(&mut conn)
    }

    fn find_all_pyth_feed_ids(&self) -> QueryResult<Vec<String>> {
        use crate::schema::coins::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let results: Vec<Option<String>> = coins
            .select(pyth_feed_id)
            .filter(pyth_feed_id.is_not_null())
            .distinct()
            .load(&mut conn)?;

        Ok(results.into_iter().flatten().collect())
    }
}
