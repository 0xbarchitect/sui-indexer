use crate::schema::shared_objects;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = shared_objects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SharedObject {
    pub id: i32,
    pub object_id: String,
    pub initial_shared_version: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = shared_objects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSharedObject {
    pub object_id: String,
    pub initial_shared_version: i64,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = shared_objects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateSharedObject {
    pub object_id: Option<String>,
    pub initial_shared_version: Option<i64>,
}
