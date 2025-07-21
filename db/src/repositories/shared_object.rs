use crate::models::shared_object::{NewSharedObject, SharedObject, UpdateSharedObject};
use crate::repositories::SharedObjectRepository;
use crate::DbPool;

use diesel::prelude::*;

pub struct SharedObjectRepositoryImpl {
    db_pool: DbPool,
}

impl SharedObjectRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        SharedObjectRepositoryImpl { db_pool }
    }
}

impl SharedObjectRepository for SharedObjectRepositoryImpl {
    fn create(&self, new_shared_object: &NewSharedObject) -> QueryResult<SharedObject> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(shared_objects)
            .values(new_shared_object)
            .get_result(&mut conn)
    }

    fn update(
        &self,
        id_val: i32,
        update_shared_object: &UpdateSharedObject,
    ) -> QueryResult<SharedObject> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(shared_objects.find(id_val))
            .set(update_shared_object)
            .get_result(&mut conn)
    }

    fn delete(&self, id_val: i32) -> QueryResult<bool> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(shared_objects.find(id_val)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, id_val: i32) -> QueryResult<SharedObject> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        shared_objects.find(id_val).get_result(&mut conn)
    }

    fn find_by_object_id(&self, object_id_val: &str) -> QueryResult<SharedObject> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        shared_objects
            .filter(object_id.eq(object_id_val))
            .first(&mut conn)
    }

    fn find_all(&self) -> QueryResult<Vec<SharedObject>> {
        use crate::schema::shared_objects::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        shared_objects.load(&mut conn)
    }
}
