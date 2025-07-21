use crate::models::liquidation_order::{
    LiquidationOrder, NewLiquidationOrder, TopLiquidationOrder, UpdateLiquidationOrder,
};
use crate::repositories::LiquidationOrderRepository;
use crate::DbPool;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;
pub struct LiquidationOrderRepositoryImpl {
    db_pool: DbPool,
}

impl LiquidationOrderRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        LiquidationOrderRepositoryImpl { db_pool }
    }
}

impl LiquidationOrderRepository for LiquidationOrderRepositoryImpl {
    fn create(&self, order: &NewLiquidationOrder) -> QueryResult<LiquidationOrder> {
        use crate::schema::liquidation_orders::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(liquidation_orders)
            .values(order)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        order_id: i32,
        order: &UpdateLiquidationOrder,
    ) -> QueryResult<LiquidationOrder> {
        use crate::schema::liquidation_orders::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(liquidation_orders.find(order_id))
            .set(order)
            .get_result(&mut conn)
    }

    fn delete(&self, order_id: i32) -> QueryResult<bool> {
        use crate::schema::liquidation_orders::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(liquidation_orders.find(order_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, order_id: i32) -> QueryResult<LiquidationOrder> {
        use crate::schema::liquidation_orders::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        liquidation_orders.find(order_id).get_result(&mut conn)
    }

    fn find_by_platform_and_address(
        &self,
        platform_val: &str,
        address_val: &str,
    ) -> QueryResult<LiquidationOrder> {
        use crate::schema::liquidation_orders::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        liquidation_orders
            .filter(platform.eq(platform_val))
            .filter(borrower.eq(address_val))
            .first(&mut conn)
    }

    fn find_top_orders_by_hf(&self, limit: i32) -> QueryResult<Vec<TopLiquidationOrder>> {
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        sql_query(
            "SELECT platform,borrower,hf,debt_coin,collateral_coin,amount_repay,amount_usd
              FROM liquidation_orders  
              WHERE updated_at >= NOW() - INTERVAL '1 minutes'
              ORDER BY hf ASC 
              LIMIT $1",
        )
        .bind::<Integer, _>(limit)
        .load(&mut conn)
    }
}
