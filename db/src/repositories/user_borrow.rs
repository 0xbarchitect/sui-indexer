use crate::models::user_borrow::{
    NewUserBorrow, UpdateUserBorrow, UserBorrow, UserBorrowCoin, UserBorrowDistinct,
    UserBorrowWithCoinInfo,
};
use crate::repositories::{UserBorrowRepository, UserDepositRepository};
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

pub struct UserBorrowRepositoryImpl {
    db_pool: DbPool,
}

impl UserBorrowRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        UserBorrowRepositoryImpl { db_pool }
    }
}

impl UserBorrowRepository for UserBorrowRepositoryImpl {
    fn create(&self, user_borrow: &NewUserBorrow) -> QueryResult<UserBorrow> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(user_borrows)
            .values(user_borrow)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        user_borrow_id: i32,
        user_borrow: &UpdateUserBorrow,
    ) -> QueryResult<UserBorrow> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(user_borrows.find(user_borrow_id))
            .set(user_borrow)
            .get_result(&mut conn)
    }

    fn delete(&self, user_borrow_id: i32) -> QueryResult<bool> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(user_borrows.find(user_borrow_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, user_borrow_id: i32) -> QueryResult<UserBorrow> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_borrows.find(user_borrow_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<UserBorrow>> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_borrows.load(&mut conn)
    }

    fn delete_by_platform_and_address(
        &self,
        platform_name: &str,
        address_str: &str,
    ) -> QueryResult<bool> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(
            user_borrows
                .filter(platform.eq(platform_name))
                .filter(borrower.eq(address_str)),
        )
        .execute(&mut conn)?;

        Ok(deleted_rows > 0)
    }

    fn find_by_platform_and_address(
        &self,
        platform_str: &str,
        address_str: &str,
    ) -> QueryResult<Vec<UserBorrow>> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        user_borrows
            .filter(platform.eq(platform_str))
            .filter(borrower.eq(address_str))
            .load(&mut conn)
    }

    fn find_by_platform_and_address_with_coin_info(
        &self,
        platform_str: &str,
        borrower_str: &str,
    ) -> QueryResult<Vec<UserBorrowWithCoinInfo>> {
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        sql_query(
            "SELECT ub.platform, ub.borrower, ub.coin_type, ub.amount, 
                    c.decimals, c.price_pyth, c.pyth_decimals, c.pyth_feed_id, c.pyth_info_object_id, c.vaa, c.navi_feed_id,
                    lm.borrow_weight, lm.liquidation_ratio, lm.liquidation_penalty, lm.liquidation_fee, lm.asset_id,
                    lm.pool_id, lm.borrow_index, lm.supply_index
             FROM user_borrows ub 
             INNER JOIN coins c ON ub.coin_type = c.coin_type
             INNER JOIN lending_markets lm on ub.platform = lm.platform AND ub.coin_type = lm.coin_type
             WHERE ub.platform = $1 AND ub.borrower = $2",
        )
        .bind::<Text, _>(platform_str)
        .bind::<Text, _>(borrower_str)
        .load(&mut conn)
    }

    fn find_distinct_platform_and_address(&self) -> QueryResult<Vec<UserBorrowDistinct>> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_borrows
            .select((platform, borrower, obligation_id))
            .distinct()
            .load(&mut conn)
    }

    fn find_coins_by_platform_and_address(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserBorrowCoin>> {
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        sql_query(
            "SELECT DISTINCT ub.coin_type
             FROM user_borrows ub
             WHERE ub.platform = $1 AND ub.borrower = $2",
        )
        .bind::<Text, _>(platform)
        .bind::<Text, _>(address)
        .load(&mut conn)
    }

    fn delete_by_platform_and_address_and_obligation_id(
        &self,
        platform_str: &str,
        address_str: &str,
        obligation_id_str: &str,
    ) -> QueryResult<bool> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(
            user_borrows
                .filter(platform.eq(platform_str))
                .filter(borrower.eq(address_str))
                .filter(obligation_id.eq(obligation_id_str)),
        )
        .execute(&mut conn)?;

        Ok(deleted_rows > 0)
    }

    fn find_by_platform_and_address_and_coin_type(
        &self,
        platform_str: &str,
        address_str: &str,
        coin_type_str: &str,
    ) -> QueryResult<UserBorrow> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_borrows
            .filter(platform.eq(platform_str))
            .filter(borrower.eq(address_str))
            .filter(coin_type.eq(coin_type_str))
            .first(&mut conn)
    }

    fn find_by_platform_and_obligation_id(
        &self,
        platform_str: &str,
        obligation_id_str: &str,
    ) -> QueryResult<UserBorrow> {
        use crate::schema::user_borrows::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_borrows
            .filter(platform.eq(platform_str))
            .filter(obligation_id.eq(obligation_id_str))
            .first(&mut conn)
    }
}
