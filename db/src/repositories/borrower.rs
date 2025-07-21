use crate::models::borrower::{Borrower, NewBorrower, UpdateBorrower};
use crate::repositories::BorrowerRepository;
use crate::DbPool;

use diesel::prelude::*;

pub struct BorrowerRepositoryImpl {
    db_pool: DbPool,
}

impl BorrowerRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        BorrowerRepositoryImpl { db_pool }
    }
}

impl BorrowerRepository for BorrowerRepositoryImpl {
    fn create(&self, new_borrower: &NewBorrower) -> QueryResult<Borrower> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(borrowers)
            .values(new_borrower)
            .get_result(&mut conn)
    }

    fn update(&self, borrower_id: i32, update_borrower: &UpdateBorrower) -> QueryResult<Borrower> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(borrowers.find(borrower_id))
            .set(update_borrower)
            .get_result(&mut conn)
    }

    fn delete(&self, borrower_id: i32) -> QueryResult<bool> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(borrowers.find(borrower_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, borrower_id: i32) -> QueryResult<Borrower> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        borrowers.find(borrower_id).get_result(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<Borrower>> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        borrowers.load(&mut conn)
    }

    fn find_by_platform_and_address(
        &self,
        platform_val: &str,
        address_val: &str,
    ) -> QueryResult<Borrower> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        borrowers
            .filter(platform.eq(platform_val))
            .filter(borrower.eq(address_val))
            .first(&mut conn)
    }

    fn find_all_by_status(&self, status_val: i32) -> QueryResult<Vec<Borrower>> {
        use crate::schema::borrowers::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        borrowers
            .filter(status.eq(status_val))
            .load(&mut conn)
            .map_err(|e| {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UnableToSendCommand,
                    Box::new(e.to_string()),
                )
            })
    }
}
