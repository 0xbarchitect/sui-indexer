use crate::{
    config::Config,
    constant,
    indexer::{self, registry::EventProcessorRegistry},
    service::{
        db_service::{lending, pool},
        registry::ServiceRegistry,
    },
    utils,
};
use db::{
    models::metric::{Metric, NewMetric, UpdateMetric},
    repositories::{
        CoinRepository, MetricRepository, PoolRepository, UserBorrowRepository,
        UserDepositRepository,
    },
};

use anyhow::Result;
use async_trait::async_trait;
use futures::{
    stream::{self, StreamExt},
    Future,
};
use prometheus::{core::Atomic, Registry};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
    {collections::HashMap, sync::Arc},
};
use sui_data_ingestion_core::{
    DataIngestionMetrics, ExecutorProgress, FileProgressStore, IndexerExecutor, ReaderOptions,
    ShimProgressStore, Worker, WorkerPool,
};
use sui_sdk::{
    rpc_types::{CheckpointId, EventFilter, SuiTransactionBlockResponseOptions},
    types::{
        digests::{Digest, TransactionDigest},
        messages_checkpoint::CheckpointSequenceNumber,
    },
    SuiClient,
};
use sui_types::{
    effects::TransactionEffectsAPI, event::Event, full_checkpoint_content::CheckpointData,
};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    time::{sleep, Duration, Instant},
};
use tokio_tungstenite::tungstenite::client;
use tracing::{debug, error, info, instrument, trace, warn};

pub async fn setup_local_reader<W: Worker + 'static>(
    worker: W,
    local_chk_path: String, // path to local directory with checkpoints
    indexer_progress_filepath: String, // path to file with indexer progress
    remote_store_url: Option<String>, // for fallback
    initial_checkpoint_number: CheckpointSequenceNumber,
    concurrency: usize,
) -> Result<(
    impl Future<Output = Result<ExecutorProgress>>,
    oneshot::Sender<()>,
)> {
    let (exit_sender, exit_receiver) = oneshot::channel();
    let metrics = DataIngestionMetrics::new(&Registry::new());

    //let progress_store = FileProgressStore::new(PathBuf::from(indexer_progress_filepath));
    let progress_store = ShimProgressStore(initial_checkpoint_number);

    let mut executor = IndexerExecutor::new(
        progress_store,
        1, /* number of workflow types */
        metrics,
    );
    let worker_pool = WorkerPool::new(worker, "local_reader".to_string(), concurrency);
    executor.register(worker_pool).await?;

    Ok((
        executor.run(
            PathBuf::from(local_chk_path), // path to a local directory
            remote_store_url,              // optional remote store URL
            vec![],                        // optional remote store access options
            ReaderOptions::default(),      /* remote_read_batch_size */
            exit_receiver,
        ),
        exit_sender,
    ))
}

#[async_trait]
impl Worker for OnchainIndexer {
    type Result = ();

    async fn process_checkpoint(&self, checkpoint: &CheckpointData) -> Result<()> {
        let start_time = Instant::now();

        let seq_number = checkpoint.checkpoint_summary.sequence_number;
        let chk_timestamp = checkpoint.checkpoint_summary.timestamp_ms;
        let lagging_timestamp_ms = utils::lagging_timestamp_ms(chk_timestamp);

        // for development purposes, scan only 1 checkpoint
        if self.config.indexer.dev_mode
            && seq_number > self.config.indexer.start_checkpoint_number + 1
        {
            return Ok(());
        }

        warn!(
            "Start processing chk #{} with timestamp {}, lagging {}ms",
            seq_number, chk_timestamp, lagging_timestamp_ms,
        );

        let event_map = self.collect_unique_events(checkpoint);
        let unique_events: Vec<_> = event_map.into_values().collect();

        info!(
            "Checkpoint #{}: collected {} unique events from transactions",
            seq_number,
            unique_events.len()
        );

        let events = if unique_events.is_empty() {
            let elapsed_time = start_time.elapsed();
            warn!(
                "Found no events in checkpoint #{} in {:?}ms",
                seq_number,
                elapsed_time.as_millis()
            );

            vec![]
        } else {
            let results = stream::iter(unique_events)
                .map(|(event, tx_digest)| async move { self.process_event(event, tx_digest).await })
                .buffer_unordered(10)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .filter_map(|result| result.ok())
                .collect::<Vec<_>>();

            let elapsed_time = start_time.elapsed();
            warn!(
                "Processed chk #{} with {} events in {:?}ms.",
                seq_number,
                results.len(),
                elapsed_time.as_millis(),
            );

            // processing time metrics
            let processing_time = elapsed_time.as_millis() as u64;

            if processing_time > self.max_processing_time.load(Ordering::SeqCst) {
                self.max_processing_time
                    .store(processing_time, Ordering::SeqCst);
            };

            if processing_time < self.min_processing_time.load(Ordering::SeqCst) {
                self.min_processing_time
                    .store(processing_time, Ordering::SeqCst);
            };

            self.total_processing_time
                .fetch_add(processing_time, Ordering::SeqCst);

            self.total_processed_checkpoints
                .fetch_add(1, Ordering::SeqCst);

            results
        };

        // lagging timestamp metrics
        let lagging_timestamp_ms = utils::lagging_timestamp_ms(chk_timestamp);

        if lagging_timestamp_ms > self.max_lagging.load(Ordering::SeqCst) {
            self.max_lagging
                .store(lagging_timestamp_ms, Ordering::SeqCst);
        };

        if lagging_timestamp_ms < self.min_lagging.load(Ordering::SeqCst) {
            self.min_lagging
                .store(lagging_timestamp_ms, Ordering::SeqCst);
        };

        self.total_lagging
            .fetch_add(lagging_timestamp_ms, Ordering::SeqCst);

        self.total_checkpoints.fetch_add(1, Ordering::SeqCst);

        // update the latest seq number and timestamp
        if seq_number > self.latest_seq_number.load(Ordering::SeqCst) {
            self.latest_seq_number.store(seq_number, Ordering::SeqCst);
        }

        if chk_timestamp > self.latest_timestamp_ms.load(Ordering::SeqCst) {
            self.latest_timestamp_ms
                .store(chk_timestamp, Ordering::SeqCst);
        }

        warn!(
            "Latest chk #{} with timestamp {}, lagging {}ms",
            self.latest_seq_number.load(Ordering::SeqCst),
            self.latest_timestamp_ms.load(Ordering::SeqCst),
            lagging_timestamp_ms,
        );

        // send alert message if lagging exceeds the threshold
        if lagging_timestamp_ms > self.config.arbitrage.indexer_lagging_ms_threshold {
            let current_timestamp = utils::get_current_timestamp_secs();
            let next_alert_timestamp = self.next_alert_timestamp.load(Ordering::SeqCst);

            if current_timestamp > next_alert_timestamp {
                let alert_backoff_factor = self.alert_backoff_factor.fetch_add(1, Ordering::SeqCst);
                let capped_factor = alert_backoff_factor.min(8);

                self.next_alert_timestamp.store(
                    current_timestamp + (1 << capped_factor) * 900,
                    Ordering::SeqCst,
                );
            }
        } else {
            // reset the alert backoff factor if lagging is within the threshold
            self.alert_backoff_factor.store(0, Ordering::SeqCst);
        }

        // save the metrics to the database for each 1K checkpoints
        if seq_number % 1_000 == 0 {
            let avg_processing_time = if self.total_processed_checkpoints.load(Ordering::SeqCst) > 0
            {
                self.total_processing_time.load(Ordering::SeqCst) as f32
                    / self.total_processed_checkpoints.load(Ordering::SeqCst) as f32
            } else {
                0.0
            };

            let avg_lagging = if self.total_checkpoints.load(Ordering::SeqCst) > 0 {
                self.total_lagging.load(Ordering::SeqCst) as f32
                    / self.total_checkpoints.load(Ordering::SeqCst) as f32
            } else {
                0.0
            };

            let new_metric = crate::types::Metric {
                latest_seq_number: seq_number as i32,
                total_checkpoints: self.total_checkpoints.load(Ordering::SeqCst) as i32,
                total_processed_checkpoints: self.total_processed_checkpoints.load(Ordering::SeqCst)
                    as i32,
                max_processing_time: self.max_processing_time.load(Ordering::SeqCst) as f32,
                min_processing_time: self.min_processing_time.load(Ordering::SeqCst) as f32,
                avg_processing_time,
                max_lagging: self.max_lagging.load(Ordering::SeqCst) as f32,
                min_lagging: self.min_lagging.load(Ordering::SeqCst) as f32,
                avg_lagging,
            };

            self.db_lending_service.save_metric_to_db(new_metric)?;
        }

        Ok(())
    }
}

pub struct OnchainIndexer {
    config: Arc<Config>,
    client: Arc<SuiClient>,
    db_pool_service: Arc<pool::PoolService>,
    db_lending_service: Arc<lending::LendingService>,
    service_registry: Arc<ServiceRegistry>,
    event_processor_registry: Arc<EventProcessorRegistry>,

    latest_seq_number: Arc<AtomicU64>,
    pub latest_timestamp_ms: Arc<AtomicU64>,
    pub start_seq_number: u64,

    total_checkpoints: Arc<AtomicU64>,
    total_processed_checkpoints: Arc<AtomicU64>,
    max_processing_time: Arc<AtomicU64>,
    min_processing_time: Arc<AtomicU64>,
    total_processing_time: Arc<AtomicU64>,
    max_lagging: Arc<AtomicU64>,
    min_lagging: Arc<AtomicU64>,
    total_lagging: Arc<AtomicU64>,

    next_alert_timestamp: Arc<AtomicU64>,
    alert_backoff_factor: Arc<AtomicU64>,
}

impl OnchainIndexer {
    pub fn new(
        config: Arc<Config>,
        client: Arc<SuiClient>,
        db_pool_service: Arc<pool::PoolService>,
        db_lending_service: Arc<lending::LendingService>,
        service_registry: Arc<ServiceRegistry>,
        event_processor_registry: Arc<EventProcessorRegistry>,
        latest_timestamp_ms: Arc<AtomicU64>,
    ) -> Self {
        let mut start_seq_number = config.indexer.start_checkpoint_number;
        let total_checkpoints = Arc::new(AtomicU64::new(0));
        let total_processed_checkpoints = Arc::new(AtomicU64::new(0));
        let max_processing_time = Arc::new(AtomicU64::new(0));
        let min_processing_time = Arc::new(AtomicU64::new(u64::MAX));
        let total_processing_time = Arc::new(AtomicU64::new(0));
        let max_lagging = Arc::new(AtomicU64::new(0));
        let min_lagging = Arc::new(AtomicU64::new(u64::MAX));
        let total_lagging = Arc::new(AtomicU64::new(0));

        if !config.indexer.dev_mode {
            if let Some(latest_checkpoint) =
                db_lending_service.find_latest_seq_number().unwrap_or(None)
            {
                // Initialize the latest sequence number and timestamp from the database
                info!(
                    "OnchainIndexer initialized with latest checkpoint #{}",
                    latest_checkpoint.latest_seq_number
                );

                start_seq_number = latest_checkpoint.latest_seq_number as u64;

                total_checkpoints
                    .store(latest_checkpoint.total_checkpoints as u64, Ordering::SeqCst);

                total_processed_checkpoints.store(
                    latest_checkpoint.total_processed_checkpoints as u64,
                    Ordering::SeqCst,
                );

                max_processing_time.store(
                    latest_checkpoint.max_processing_time as u64,
                    Ordering::SeqCst,
                );

                if latest_checkpoint.min_processing_time > 0.0 {
                    min_processing_time.store(
                        latest_checkpoint.min_processing_time as u64,
                        Ordering::SeqCst,
                    );
                }

                total_processing_time.store(
                    (latest_checkpoint.total_processed_checkpoints as f64
                        * latest_checkpoint.avg_processing_time as f64) as u64,
                    Ordering::SeqCst,
                );

                max_lagging.store(latest_checkpoint.max_lagging as u64, Ordering::SeqCst);

                if latest_checkpoint.min_lagging > 0.0 {
                    min_lagging.store(latest_checkpoint.min_lagging as u64, Ordering::SeqCst);
                }

                total_lagging.store(
                    (latest_checkpoint.total_checkpoints as f64
                        * latest_checkpoint.avg_lagging as f64) as u64,
                    Ordering::SeqCst,
                );
            }
        }

        let latest_seq_number = Arc::new(AtomicU64::new(start_seq_number));

        OnchainIndexer {
            config,
            client,
            db_pool_service,
            db_lending_service,
            service_registry,
            event_processor_registry,
            latest_seq_number,
            latest_timestamp_ms,
            start_seq_number,
            total_checkpoints,
            total_processed_checkpoints,
            max_processing_time,
            min_processing_time,
            total_processing_time,
            max_lagging,
            min_lagging,
            total_lagging,
            next_alert_timestamp: Arc::new(AtomicU64::new(0)),
            alert_backoff_factor: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Process a single event in checkpoint data.
    ///
    async fn process_event(
        &self,
        event: Event,
        tx_digest: String,
    ) -> Result<indexer::OnchainEvent> {
        let start = Instant::now();

        let event_type = event.type_.to_string();

        let processed_event = self
            .event_processor_registry
            .process_raw_event(event, &tx_digest)
            .await
            .map_err(|e| {
                error!("failed to process event: {}: {}", event_type, e);
                e
            })?;

        let elapsed = start.elapsed();
        info!("Processed event {:?} in {:?}", event_type, elapsed);

        Ok(processed_event)
    }

    /// Process transaction events by tx_digest.
    ///
    pub async fn process_tx_events(&self, tx_digest: &str) -> Result<()> {
        let tx_digest = TransactionDigest::from_str(tx_digest)
            .map_err(|_| anyhow::anyhow!("Failed to parse transaction digest: {}", tx_digest))?;

        let options = sui_sdk::rpc_types::SuiTransactionBlockResponseOptions {
            show_input: true,
            show_raw_input: true,
            show_effects: true,
            show_raw_effects: true,
            show_events: true,
            show_object_changes: true,
            show_balance_changes: true,
        };
        let tx = self
            .client
            .read_api()
            .get_transaction_with_options(tx_digest, options)
            .await?;

        if let Some(events) = tx.events {
            for event in events.data {
                let start = Instant::now();
                let event_type = event.type_.clone();

                match self
                    .event_processor_registry
                    .process_tx_event(event, &tx_digest.to_string())
                    .await
                {
                    Ok(_) => {
                        let elapsed = start.elapsed();
                        info!(
                            "Processed event {} in {:?}ms",
                            event_type,
                            elapsed.as_millis()
                        );
                    }
                    Err(e) => {
                        error!("Failed to process event: {}: {}", event_type, e);
                        continue;
                    }
                }
            }
        } else {
            info!("No events found for transaction {:?}", tx_digest);
        }

        Ok(())
    }

    /// helper method to extract unique events
    /// from checkpoint transactions and return a map of event type to a tuple of (event, transaction_digest)
    fn collect_unique_events(
        &self,
        checkpoint: &CheckpointData,
    ) -> HashMap<String, (Event, String)> {
        let mut event_map = HashMap::new();

        for tx in &checkpoint.transactions {
            let Some(tx_events) = &tx.events else {
                continue;
            };

            for event in &tx_events.data {
                if let Ok(event_type) = self.event_processor_registry.get_event_id(event) {
                    // Only clone when inserting - replaces older events of same type with newer ones
                    let tx_digest = tx.effects.transaction_digest().to_string();
                    info!(
                        "insert event with type {} from tx {} to the checkpoint map",
                        event_type, tx_digest
                    );
                    event_map.insert(event_type, (event.clone(), tx_digest));
                }
            }
        }

        event_map
    }
}
