use mev_lib::{
    indexer::{onchain_indexer::OnchainIndexer, registry::EventProcessorRegistry},
    service::registry::ServiceRegistry,
    utils,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{str::FromStr, sync::Arc};
use sui_sdk::{
    rpc_types::{CheckpointId, EventFilter, SuiTransactionBlockResponseOptions},
    types::{
        digests::{Digest, TransactionDigest},
        messages_checkpoint::CheckpointSequenceNumber,
    },
    SuiClient,
};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Subcommand)]
pub enum IndexCommands {
    #[command(about = "Get events logs of a transaction")]
    TxEvents {
        #[arg(long)]
        digest: String,
    },

    #[command(about = "Get transaction details response")]
    TxDetails {
        #[arg(long)]
        digest: String,
    },

    #[command(about = "Get checkpoint details")]
    CheckpointDetails {
        #[arg(long)]
        checkpoint_number: u64,
    },
}

//handlers
pub async fn handle_query_events(client: Arc<SuiClient>, digest: &str) -> Result<()> {
    let tx_digest = TransactionDigest::from_str(digest)
        .map_err(|_| anyhow::anyhow!("Failed to parse transaction digest: {}", digest))?;

    let query = EventFilter::Transaction(tx_digest);
    let events = client
        .event_api()
        .query_events(query, None, None, false)
        .await?;

    for event in events.data {
        info!("Event type {:?}", event.type_);
        info!("Event data {:?}", event.parsed_json);
    }

    Ok(())
}

pub async fn handle_query_tx(onchain_indexer: Arc<OnchainIndexer>, digest: &str) -> Result<()> {
    onchain_indexer.process_tx_events(digest).await
}

pub async fn handle_query_checkpoint(client: Arc<SuiClient>, checkpoint_number: u64) -> Result<()> {
    let checkpoint_seq_num: CheckpointSequenceNumber = checkpoint_number;
    let checkpoint_id = CheckpointId::from(checkpoint_seq_num);

    let checkpoint = client.read_api().get_checkpoint(checkpoint_id).await?;

    info!("Checkpoint {:?}", checkpoint);
    Ok(())
}
