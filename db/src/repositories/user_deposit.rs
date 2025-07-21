use crate::models::user_deposit::{
    NewUserDeposit, UpdateUserDeposit, UserDeposit, UserDepositDistinct, UserDepositWithCoinInfo,
};
use crate::repositories::UserDepositRepository;
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

pub struct UserDepositRepositoryImpl {
    db_pool: DbPool,
}

impl UserDepositRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        UserDepositRepositoryImpl { db_pool }
    }
}

impl UserDepositRepository for UserDepositRepositoryImpl {
    fn create(&self, user_deposit: &NewUserDeposit) -> QueryResult<UserDeposit> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(user_deposits)
            .values(user_deposit)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        user_deposit_id: i32,
        user_deposit: &UpdateUserDeposit,
    ) -> QueryResult<UserDeposit> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(user_deposits.find(user_deposit_id))
            .set(user_deposit)
            .get_result(&mut conn)
    }

    fn delete(&self, user_deposit_id: i32) -> QueryResult<bool> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows =
            diesel::delete(user_deposits.find(user_deposit_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, user_deposit_id: i32) -> QueryResult<UserDeposit> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_deposits.find(user_deposit_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<UserDeposit>> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_deposits.load(&mut conn)
    }

    fn delete_by_platform_and_address(
        &self,
        platform_name: &str,
        address: &str,
    ) -> QueryResult<bool> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(
            user_deposits
                .filter(platform.eq(platform_name))
                .filter(borrower.eq(address)),
        )
        .execute(&mut conn)?;

        Ok(deleted_rows > 0)
    }

    fn find_by_platform_and_address(
        &self,
        platform_str: &str,
        borrower_str: &str,
    ) -> QueryResult<Vec<UserDeposit>> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;
        user_deposits
            .filter(platform.eq(platform_str))
            .filter(borrower.eq(borrower_str))
            .load(&mut conn)
    }

    fn find_by_platform_and_address_and_coin_type(
        &self,
        platform_str: &str,
        address_str: &str,
        coin_type_str: &str,
    ) -> QueryResult<UserDeposit> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_deposits
            .filter(platform.eq(platform_str))
            .filter(borrower.eq(address_str))
            .filter(coin_type.eq(coin_type_str))
            .first(&mut conn)
    }

    fn find_by_platform_and_address_with_coin_info(
        &self,
        platform_str: &str,
        borrower_str: &str,
    ) -> QueryResult<Vec<UserDepositWithCoinInfo>> {
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        sql_query(
            "SELECT ud.platform, ud.borrower, ud.coin_type, ud.amount, 
                    c.decimals, c.price_pyth, c.pyth_decimals, c.pyth_feed_id, c.pyth_info_object_id, c.vaa, c.navi_feed_id,
                    lm.liquidation_threshold, lm.asset_id, lm.pool_id, lm.borrow_index, lm.supply_index, 
                    lm.ctoken_supply, lm.available_amount, lm.borrowed_amount, lm.unclaimed_spread_fees
             FROM user_deposits ud 
             INNER JOIN coins c ON ud.coin_type = c.coin_type
             INNER JOIN lending_markets lm on ud.platform = lm.platform AND ud.coin_type = lm.coin_type
             WHERE ud.platform = $1 AND ud.borrower = $2",
        )
        .bind::<Text, _>(platform_str)
        .bind::<Text, _>(borrower_str)
        .load(&mut conn)
    }

    fn delete_by_platform_and_address_and_obligation_id(
        &self,
        platform_str: &str,
        borrower_str: &str,
        obligation_id_str: &str,
    ) -> QueryResult<bool> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(
            user_deposits
                .filter(platform.eq(platform_str))
                .filter(borrower.eq(borrower_str))
                .filter(obligation_id.eq(obligation_id_str)),
        )
        .execute(&mut conn)?;

        Ok(deleted_rows > 0)
    }

    fn find_distinct_platform_and_address(&self) -> QueryResult<Vec<UserDepositDistinct>> {
        use crate::schema::user_deposits::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        user_deposits
            .select((platform, borrower, obligation_id))
            .distinct()
            .load(&mut conn)
    }
}
