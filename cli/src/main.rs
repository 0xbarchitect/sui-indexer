use db::models::pool::{self, NewPool, Pool, UpdatePool};
use db::repositories::{
    borrower::BorrowerRepositoryImpl, coin::CoinRepositoryImpl, metric::MetricRepositoryImpl,
    pool::PoolRepositoryImpl, pool_tick::PoolTickRepositoryImpl,
    shared_object::SharedObjectRepositoryImpl, user_borrow::UserBorrowRepositoryImpl,
    user_deposit::UserDepositRepositoryImpl, BorrowerRepository, CoinRepository, MetricRepository,
    PoolRepository, PoolTickRepository, SharedObjectRepository, UserBorrowRepository,
    UserDepositRepository,
};
use db::{establish_connection_pool, run_migrations};
use mev_lib::{
    config::Config,
    indexer::{onchain_indexer::OnchainIndexer, registry::EventProcessorRegistry},
    service::{
        db_service::{lending::LendingService, pool::PoolService},
        dex::DEXService,
        registry::ServiceRegistry,
    },
    types::Borrower,
    utils::{self, ptb::PTBHelper},
};

mod index_cmd;

use index_cmd::IndexCommands;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::sync::{atomic::AtomicU64, Arc, Mutex};
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::{SuiClient, SuiClientBuilder};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, trace, warn, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(name = "indexer-cli")]
#[command(about = "SUI Indexer CLI")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Index onchain data")]
    Index {
        #[command(subcommand)]
        command: IndexCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Config::load_toml()?);

    let log_level = utils::convert_log_level_to_tracing_level(&config.log_level);
    let filter = EnvFilter::from_default_env().add_directive(log_level.into());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()?;

    warn!("Starting mev-cli...");

    let db_conn = establish_connection_pool(
        &config.database.database_url,
        config.database.db_connection_pool_max_size,
        config.database.db_connection_pool_idle_size,
    )?;
    warn!("Connected to database {}", &config.database.database_url);

    run_migrations(&db_conn)?;
    warn!("Database migrations completed");

    let pool_repo: Arc<dyn PoolRepository + Send + Sync> =
        Arc::new(PoolRepositoryImpl::new(db_conn.clone()));

    let coin_repo: Arc<dyn CoinRepository + Send + Sync> =
        Arc::new(CoinRepositoryImpl::new(db_conn.clone()));

    let user_borrow_repo: Arc<dyn UserBorrowRepository + Send + Sync> =
        Arc::new(UserBorrowRepositoryImpl::new(db_conn.clone()));

    let user_deposit_repo: Arc<dyn UserDepositRepository + Send + Sync> =
        Arc::new(UserDepositRepositoryImpl::new(db_conn.clone()));

    let pool_tick_repo: Arc<dyn PoolTickRepository + Send + Sync> =
        Arc::new(PoolTickRepositoryImpl::new(db_conn.clone()));

    let borrower_repo: Arc<dyn BorrowerRepository + Send + Sync> =
        Arc::new(BorrowerRepositoryImpl::new(db_conn.clone()));

    let metric_repo: Arc<dyn MetricRepository + Send + Sync> =
        Arc::new(MetricRepositoryImpl::new(db_conn.clone()));

    let shared_object_repo: Arc<dyn SharedObjectRepository + Send + Sync> =
        Arc::new(SharedObjectRepositoryImpl::new(db_conn.clone()));

    let network_config = config.networks.get(&config.run_mode).unwrap();

    let sui_client = Arc::new(
        SuiClientBuilder::default()
            .build(network_config.rpc_url.clone())
            .await?,
    );
    warn!(
        "Sui client initialized with RPC URL: {}",
        network_config.rpc_url,
    );

    // register services
    let db_pool_service = Arc::new(PoolService::new(
        Arc::clone(&config),
        Arc::clone(&pool_repo),
        Arc::clone(&coin_repo),
        Arc::clone(&pool_tick_repo),
    ));

    let db_lending_service = Arc::new(LendingService::new(
        Arc::clone(&config),
        Arc::clone(&coin_repo),
        Arc::clone(&user_borrow_repo),
        Arc::clone(&user_deposit_repo),
        Arc::clone(&borrower_repo),
        Arc::clone(&metric_repo),
        Arc::clone(&shared_object_repo),
    ));

    let ptb_helper = Arc::new(PTBHelper::new(
        Arc::clone(&sui_client),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
    ));

    let service_registry = Arc::new(ServiceRegistry::new(
        Arc::clone(&config),
        Arc::clone(&sui_client),
        Arc::clone(&coin_repo),
        Arc::clone(&pool_repo),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
        Arc::clone(&ptb_helper),
    ));

    // event-processor registry
    let event_processor_registry = Arc::new(EventProcessorRegistry::new(
        Arc::clone(&config),
        Arc::clone(&sui_client),
        Arc::clone(&pool_repo),
        Arc::clone(&coin_repo),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
        Arc::clone(&service_registry),
    ));

    // onchain indexer
    let latest_timestamp_ms = Arc::new(AtomicU64::new(0));

    let onchain_indexer = Arc::new(OnchainIndexer::new(
        Arc::clone(&config),
        Arc::clone(&sui_client),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
        Arc::clone(&service_registry),
        Arc::clone(&event_processor_registry),
        Arc::clone(&latest_timestamp_ms),
    ));

    let args = Cli::parse();
    match args.command {
        Commands::Index { command } => match command {
            IndexCommands::TxEvents { digest } => {
                info!("Querying events for transaction: {}", digest);

                index_cmd::handle_query_events(Arc::clone(&sui_client), &digest).await?;
            }
            IndexCommands::TxDetails { digest } => {
                info!("Querying transaction details: {}", digest);

                index_cmd::handle_query_tx(Arc::clone(&onchain_indexer), &digest).await?;
            }
            IndexCommands::CheckpointDetails { checkpoint_number } => {
                info!("Querying checkpoint details: {}", checkpoint_number);

                index_cmd::handle_query_checkpoint(Arc::clone(&sui_client), checkpoint_number)
                    .await?;
            }
        },
    }

    Ok(())
}
