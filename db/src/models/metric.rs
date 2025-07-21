use crate::schema::metrics;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name = metrics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Metric {
    pub id: i32,
    pub latest_seq_number: i32,
    pub total_checkpoints: i32,
    pub total_processed_checkpoints: i32,
    pub max_processing_time: f32,
    pub min_processing_time: f32,
    pub avg_processing_time: f32,
    pub max_lagging: f32,
    pub min_lagging: f32,
    pub avg_lagging: f32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = metrics)]
pub struct NewMetric {
    pub latest_seq_number: i32,
    pub total_checkpoints: i32,
    pub total_processed_checkpoints: i32,
    pub max_processing_time: f32,
    pub min_processing_time: f32,
    pub avg_processing_time: f32,
    pub max_lagging: f32,
    pub min_lagging: f32,
    pub avg_lagging: f32,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = metrics)]
pub struct UpdateMetric {
    pub latest_seq_number: Option<i32>,
    pub total_checkpoints: Option<i32>,
    pub total_processed_checkpoints: Option<i32>,
    pub max_processing_time: Option<f32>,
    pub min_processing_time: Option<f32>,
    pub avg_processing_time: Option<f32>,
    pub max_lagging: Option<f32>,
    pub min_lagging: Option<f32>,
    pub avg_lagging: Option<f32>,
}
