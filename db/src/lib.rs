pub mod models;
pub mod repositories;
pub mod schema;

use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool(
    database_url: &str,
    max_size: usize,
    idle_size: usize,
) -> Result<DbPool> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let db_pool = Pool::builder()
        .max_size(max_size as u32)
        .min_idle(Some(idle_size as u32))
        .build(manager)
        .map_err(|e| anyhow!("Failed to create pool: {}", e))?;

    Ok(db_pool)
}

pub fn run_migrations(db_pool: &DbPool) -> Result<()> {
    //use diesel_migrations::run_pending_migrations;

    let mut conn = db_pool
        .get()
        .map_err(|e| anyhow!("Failed to get connection from pool: {}", e))?;

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!("Failed to run migrations: {}", e))?;

    Ok(())
}
