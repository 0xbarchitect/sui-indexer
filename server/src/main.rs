use mev_lib::{
    config::Config,
    indexer::{
        self,
        onchain_indexer::{self, OnchainIndexer},
        oracle::pyth,
        registry::EventProcessorRegistry,
    },
    service::{
        db_service::{lending::LendingService, pool::PoolService},
        registry::ServiceRegistry,
    },
    types::Borrower,
    utils::{self, ptb::PTBHelper},
};

use db::repositories::{
    borrower::BorrowerRepositoryImpl, coin::CoinRepositoryImpl, metric::MetricRepositoryImpl,
    pool::PoolRepositoryImpl, pool_tick::PoolTickRepositoryImpl,
    shared_object::SharedObjectRepositoryImpl, user_borrow::UserBorrowRepositoryImpl,
    user_deposit::UserDepositRepositoryImpl, BorrowerRepository, CoinRepository, MetricRepository,
    PoolRepository, PoolTickRepository, SharedObjectRepository, UserBorrowRepository,
    UserDepositRepository,
};
use db::{establish_connection_pool, run_migrations};

use anyhow::Result;
use futures::future;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use sui_data_ingestion_core::setup_single_workflow;
use sui_sdk::SuiClientBuilder;
use tokio::{
    self,
    sync::{mpsc, RwLock},
    time::{sleep, Duration},
};
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Config::load_toml()?);

    let log_level = utils::convert_log_level_to_tracing_level(&config.log_level);
    let filter = EnvFilter::from_default_env().add_directive(log_level.into());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()?;

    warn!("Starting server...");

    // connect database

    let db_conn = establish_connection_pool(
        &config.database.database_url,
        config.database.db_connection_pool_max_size,
        config.database.db_connection_pool_idle_size,
    )?;
    warn!("Connected to database {}", &config.database.database_url);

    // run db migrations
    run_migrations(&db_conn)?;
    warn!("Database migrations completed");

    // initialize db repositories
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

    let metric_repo: Arc<dyn MetricRepository + Send + Sync> =
        Arc::new(MetricRepositoryImpl::new(db_conn.clone()));

    let borrower_repo: Arc<dyn BorrowerRepository + Send + Sync> =
        Arc::new(BorrowerRepositoryImpl::new(db_conn.clone()));

    let shared_object_repo: Arc<dyn SharedObjectRepository + Send + Sync> =
        Arc::new(SharedObjectRepositoryImpl::new(db_conn.clone()));

    // initialize sui client
    let network_config = config.networks.get(&config.run_mode).unwrap();
    let rpc_url = network_config.rpc_url.as_deref().unwrap();

    let sui_client = Arc::new(SuiClientBuilder::default().build(rpc_url).await?);
    warn!("Sui client initialized with RPC URL: {}", rpc_url);

    // services
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

    let event_processor_registry = Arc::new(EventProcessorRegistry::new(
        Arc::clone(&config),
        Arc::clone(&sui_client),
        Arc::clone(&pool_repo),
        Arc::clone(&coin_repo),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
        Arc::clone(&service_registry),
    ));

    // Onchain indexer
    let latest_timestamp_ms = Arc::new(AtomicU64::new(0));

    let onchain_indexer = OnchainIndexer::new(
        Arc::clone(&config),
        Arc::clone(&sui_client),
        Arc::clone(&db_pool_service),
        Arc::clone(&db_lending_service),
        Arc::clone(&service_registry),
        Arc::clone(&event_processor_registry),
        Arc::clone(&latest_timestamp_ms),
    );

    // Task for starting Onchain indexer
    let (onchain_task, exit_sender) = if config.onchain_indexer_enabled {
        // start the onchain indexer
        // term sender MUST be kept in process lifecycle
        // and can be used to gracefully terminate the indexer
        // by sending a signal to the indexer task
        // e.g.:
        // ```
        // if term_sender.send(()).is_ok() {
        //    error!("onchain indexer terminated");
        // }
        // ```

        // DISABLE: remote reader
        let (onchain_indexing, exit_sender) = setup_single_workflow(
            onchain_indexer,
            "https://checkpoints.mainnet.sui.io".to_string(),
            config.indexer.start_checkpoint_number, /* initial checkpoint number */
            config.indexer.indexer_worker_count,    /* concurrency */
            None,                                   /* extra reader options */
        )
        .await?;

        // ENABLE: Setup local reader

        // let remote_store_url = if config.indexer.use_remote_store {
        //     config
        //         .networks
        //         .get(&config.run_mode)
        //         .and_then(|n| n.remote_store_url.clone())
        // } else {
        //     None
        // };

        // let start_seq_number = onchain_indexer.start_seq_number + 1;
        // warn!("Start seq number {}", start_seq_number);

        // let (onchain_indexing, exit_sender) = onchain_indexer::setup_local_reader(
        //     onchain_indexer,
        //     config.indexer.local_checkpoint_dir.clone(),
        //     config.indexer.indexer_progress_filepath.clone(),
        //     remote_store_url,                    /* optional remote store URL */
        //     start_seq_number,                    /* initial checkpoint number */
        //     config.indexer.indexer_worker_count, /* concurrency */
        // )
        // .await?;

        (
            tokio::spawn(async move {
                if let Err(e) = onchain_indexing.await {
                    error!("Onchain indexer failed: {:?}", e);
                }
            }),
            exit_sender,
        )
    } else {
        let (exit_sender, _exit_receiver) = tokio::sync::oneshot::channel();

        (
            tokio::spawn(async {
                future::pending::<()>().await;
            }),
            exit_sender,
        )
    };

    // running all tasks concurrently
    tokio::select! {
        _ = onchain_task => {
            info!("Onchain indexing task completed");
        }

        _ = tokio::signal::ctrl_c() => {
            warn!("Received Ctrl+C signal, shutting down...");
        }
    }

    Ok(())
}
