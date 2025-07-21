use crate::models::metric::{Metric, NewMetric, UpdateMetric};
use crate::repositories::MetricRepository;
use crate::DbPool;

use diesel::prelude::*;

pub struct MetricRepositoryImpl {
    db_pool: DbPool,
}

impl MetricRepositoryImpl {
    pub fn new(db_pool: DbPool) -> Self {
        MetricRepositoryImpl { db_pool }
    }
}

impl MetricRepository for MetricRepositoryImpl {
    fn create(&self, metric: &NewMetric) -> QueryResult<Metric> {
        use crate::schema::metrics::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::insert_into(metrics)
            .values(metric)
            .get_result(&mut conn)
    }

    fn update(&self, metric_id: i32, metric: &UpdateMetric) -> QueryResult<Metric> {
        use crate::schema::metrics::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        diesel::update(metrics.find(metric_id))
            .set(metric)
            .get_result(&mut conn)
    }

    fn delete(&self, metric_id: i32) -> QueryResult<bool> {
        use crate::schema::metrics::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        let deleted_rows = diesel::delete(metrics.find(metric_id)).execute(&mut conn)?;
        Ok(deleted_rows > 0)
    }

    fn find_by_id(&self, metric_id: i32) -> QueryResult<Metric> {
        use crate::schema::metrics::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        metrics.find(metric_id).get_result(&mut conn)
    }

    fn find_latest_seq_number(&self) -> QueryResult<Option<Metric>> {
        use crate::schema::metrics::dsl::*;
        let mut conn = self.db_pool.get().map_err(|e| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UnableToSendCommand,
                Box::new(e.to_string()),
            )
        })?;

        metrics
            .order(latest_seq_number.desc())
            .first::<Metric>(&mut conn)
            .optional()
            .map_err(|e| {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UnableToSendCommand,
                    Box::new(e.to_string()),
                )
            })
    }
}
