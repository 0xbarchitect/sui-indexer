use crate::schema::borrowers;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = borrowers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Borrower {
    pub id: i32,
    pub platform: String,
    pub borrower: String,
    pub obligation_id: Option<String>,
    pub status: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = borrowers)]
pub struct NewBorrower {
    pub platform: String,
    pub borrower: String,
    pub obligation_id: Option<String>,
    pub status: i32,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = borrowers)]
pub struct UpdateBorrower {
    pub platform: Option<String>,
    pub borrower: Option<String>,
    pub obligation_id: Option<String>,
    pub status: Option<i32>,
}
