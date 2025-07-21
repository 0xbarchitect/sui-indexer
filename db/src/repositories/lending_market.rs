use crate::models::lending_market::{
    LendingMarket, LendingMarketWithCoinInfo, NewLendingMarket, UpdateLendingMarket,
};
use crate::repositories::LendingMarketRepository;
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

pub struct LendingMarketRepositoryImpl {
    db_pool: DbPool,
}

impl LendingMarketRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}

impl LendingMarketRepository for LendingMarketRepositoryImpl {
    fn create(&self, lending_market: &NewLendingMarket) -> QueryResult<LendingMarket> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(lending_markets)
            .values(lending_market)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        market_id: i32,
        lending_market: &UpdateLendingMarket,
    ) -> QueryResult<LendingMarket> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(lending_markets.find(market_id))
            .set(lending_market)
            .get_result(&mut conn)
    }

    fn delete(&self, market_id: i32) -> QueryResult<bool> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(lending_markets.find(market_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, market_id: i32) -> QueryResult<LendingMarket> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        lending_markets.find(market_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<LendingMarket>> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        lending_markets.load(&mut conn)
    }

    fn find_by_platform_and_coin_type(
        &self,
        platform_str: &str,
        coin_type_str: &str,
    ) -> QueryResult<LendingMarket> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        lending_markets
            .filter(platform.eq(platform_str))
            .filter(coin_type.eq(coin_type_str))
            .limit(1)
            .get_result(&mut conn)
    }

    fn find_by_platform_and_coin_type_with_coin_info(
        &self,
        platform_str: &str,
        coin_type_str: &str,
    ) -> QueryResult<LendingMarketWithCoinInfo> {
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        sql_query(
            "SELECT lm.platform, lm.coin_type, lm.ltv, lm.liquidation_threshold,
                    lm.borrow_weight, lm.liquidation_ratio, lm.liquidation_penalty, lm.liquidation_fee,
                    lm.asset_id, lm.pool_id, 
                    c.decimals, c.price_pyth, c.pyth_decimals, c.pyth_info_object_id
             FROM lending_markets lm 
             LEFT JOIN coins c ON lm.coin_type = c.coin_type
             WHERE lm.platform = $1 AND lm.coin_type = $2",
        )
        .bind::<Text, _>(platform_str)
        .bind::<Text, _>(coin_type_str)
        .get_result(&mut conn)
    }

    fn find_by_platform_and_asset_id(
        &self,
        platform_val: &str,
        asset_id_val: i32,
    ) -> QueryResult<LendingMarket> {
        use crate::schema::lending_markets::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        lending_markets
            .filter(platform.eq(platform_val))
            .filter(asset_id.eq(asset_id_val))
            .first(&mut conn)
    }
}
