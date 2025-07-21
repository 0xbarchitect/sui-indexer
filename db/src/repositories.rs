pub mod borrower;
pub mod coin;
pub mod metric;
pub mod pool;
pub mod pool_tick;
pub mod shared_object;
pub mod user_borrow;
pub mod user_deposit;

use crate::models::{
    borrower::{Borrower, NewBorrower, UpdateBorrower},
    coin::{Coin, NewCoin, UpdateCoin},
    metric::{Metric, NewMetric, UpdateMetric},
    pool::{NewPool, Pool, UpdatePool},
    pool_tick::{NewPoolTick, PoolTick, UpdatePoolTick},
    shared_object::{NewSharedObject, SharedObject, UpdateSharedObject},
    user_borrow::{
        NewUserBorrow, UpdateUserBorrow, UserBorrow, UserBorrowCoin, UserBorrowDistinct,
        UserBorrowWithCoinInfo,
    },
    user_deposit::{
        NewUserDeposit, UpdateUserDeposit, UserDeposit, UserDepositDistinct,
        UserDepositWithCoinInfo,
    },
};

use diesel::prelude::*;

pub trait PoolRepository {
    fn create(&self, pool: &NewPool) -> QueryResult<Pool>;
    fn update(&self, id: i32, pool: &UpdatePool) -> QueryResult<Pool>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<Pool>;
    fn find_by_address(&self, address: &str) -> QueryResult<Pool>;
    fn find_all(&self) -> QueryResult<Vec<Pool>>;
}

pub trait CoinRepository {
    fn create(&self, coin: &NewCoin) -> QueryResult<Coin>;
    fn update(&self, id: i32, coin: &UpdateCoin) -> QueryResult<Coin>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<Coin>;
    fn find_all(&self) -> QueryResult<Vec<Coin>>;
    fn find_by_coin_type(&self, coin_type: &str) -> QueryResult<Coin>;
    fn find_by_pyth_feed_id(&self, pyth_feed_id: &str) -> QueryResult<Vec<Coin>>;
    fn find_by_navi_asset_id(&self, asset_id: i32) -> QueryResult<Coin>;
    fn find_all_pyth_feed_ids(&self) -> QueryResult<Vec<String>>;
}

pub trait UserBorrowRepository {
    fn create(&self, user_borrow: &NewUserBorrow) -> QueryResult<UserBorrow>;
    fn update(&self, id: i32, user_borrow: &UpdateUserBorrow) -> QueryResult<UserBorrow>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<UserBorrow>;
    fn find_all(&self) -> QueryResult<Vec<UserBorrow>>;

    fn delete_by_platform_and_address(&self, platform: &str, address: &str) -> QueryResult<bool>;

    fn find_by_platform_and_address(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserBorrow>>;

    fn find_by_platform_and_address_with_coin_info(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserBorrowWithCoinInfo>>;

    fn find_distinct_platform_and_address(&self) -> QueryResult<Vec<UserBorrowDistinct>>;

    fn find_coins_by_platform_and_address(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserBorrowCoin>>;

    fn delete_by_platform_and_address_and_obligation_id(
        &self,
        platform: &str,
        address: &str,
        obligation_id: &str,
    ) -> QueryResult<bool>;

    fn find_by_platform_and_address_and_coin_type(
        &self,
        platform: &str,
        address: &str,
        coin_type: &str,
    ) -> QueryResult<UserBorrow>;

    fn find_by_platform_and_obligation_id(
        &self,
        platform: &str,
        obligation_id: &str,
    ) -> QueryResult<UserBorrow>;
}

pub trait UserDepositRepository {
    fn create(&self, user_deposit: &NewUserDeposit) -> QueryResult<UserDeposit>;
    fn update(&self, id: i32, user_deposit: &UpdateUserDeposit) -> QueryResult<UserDeposit>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<UserDeposit>;
    fn find_all(&self) -> QueryResult<Vec<UserDeposit>>;

    fn delete_by_platform_and_address(&self, platform: &str, address: &str) -> QueryResult<bool>;

    fn find_by_platform_and_address(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserDeposit>>;

    fn find_by_platform_and_address_and_coin_type(
        &self,
        platform: &str,
        address: &str,
        coin_type: &str,
    ) -> QueryResult<UserDeposit>;

    fn find_by_platform_and_address_with_coin_info(
        &self,
        platform: &str,
        address: &str,
    ) -> QueryResult<Vec<UserDepositWithCoinInfo>>;

    fn delete_by_platform_and_address_and_obligation_id(
        &self,
        platform: &str,
        address: &str,
        obligation_id: &str,
    ) -> QueryResult<bool>;

    fn find_distinct_platform_and_address(&self) -> QueryResult<Vec<UserDepositDistinct>>;
}

pub trait PoolTickRepository {
    fn create(&self, pool_tick: &NewPoolTick) -> QueryResult<PoolTick>;
    fn update(&self, id: i32, pool_tick: &UpdatePoolTick) -> QueryResult<PoolTick>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<PoolTick>;
    fn find_all(&self) -> QueryResult<Vec<PoolTick>>;
    fn find_by_address_and_tick_index(
        &self,
        address: &str,
        tick_index: i32,
    ) -> QueryResult<PoolTick>;
    fn find_by_address(&self, address: &str) -> QueryResult<Vec<PoolTick>>;
    fn find_lower_tick_for_address(
        &self,
        address: &str,
        tick_index: i32,
    ) -> QueryResult<Option<PoolTick>>;

    fn find_higher_tick_for_address(
        &self,
        address: &str,
        tick_index: i32,
    ) -> QueryResult<Option<PoolTick>>;
}

pub trait MetricRepository {
    fn create(&self, metric: &NewMetric) -> QueryResult<Metric>;
    fn update(&self, id: i32, metric: &UpdateMetric) -> QueryResult<Metric>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<Metric>;
    fn find_latest_seq_number(&self) -> QueryResult<Option<Metric>>;
}

pub trait BorrowerRepository {
    fn create(&self, borrower: &NewBorrower) -> QueryResult<Borrower>;
    fn update(&self, id: i32, borrower: &UpdateBorrower) -> QueryResult<Borrower>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<Borrower>;
    fn find_all(&self) -> QueryResult<Vec<Borrower>>;
    fn find_by_platform_and_address(&self, platform: &str, address: &str) -> QueryResult<Borrower>;
    fn find_all_by_status(&self, status: i32) -> QueryResult<Vec<Borrower>>;
}

pub trait SharedObjectRepository {
    fn create(&self, shared_object: &NewSharedObject) -> QueryResult<SharedObject>;
    fn update(&self, id: i32, shared_object: &UpdateSharedObject) -> QueryResult<SharedObject>;
    fn delete(&self, id: i32) -> QueryResult<bool>;
    fn find_by_id(&self, id: i32) -> QueryResult<SharedObject>;
    fn find_by_object_id(&self, object_id: &str) -> QueryResult<SharedObject>;
    fn find_all(&self) -> QueryResult<Vec<SharedObject>>;
}
