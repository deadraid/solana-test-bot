// src/main.rs

use dotenv::dotenv;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    bench::Bench,
    config::PingThingsArgs,
    geyser::{GeyserResult, YellowstoneGrpcGeyser, YellowstoneGrpcGeyserClient},
    meteora::controller::MeteoraController,
};

mod bench;
mod config;
mod core;
mod geyser;
mod meteora;
mod tx_senders;

#[tokio::main]
async fn main() -> GeyserResult<()> {
    // Initialize default subscriber without additional filtering
    tracing_subscriber::fmt::init();

    // Load .env variables, if any
    dotenv().ok();

    // Parse CLI/config arguments
    let config: PingThingsArgs = PingThingsArgs::new();
    let bench = Bench::new(config.clone());
    let meteora = MeteoraController::new(config.clone(), bench.clone());

    info!("Starting with config: {:?}", config);

    // Build account & transaction filters
    let account_filters: HashMap<
        String,
        yellowstone_grpc_proto::geyser::SubscribeRequestFilterAccounts,
    > = HashMap::new();

    let mut tx_filters: HashMap<
        String,
        yellowstone_grpc_proto::geyser::SubscribeRequestFilterTransactions,
    > = HashMap::new();
    tx_filters.insert(
        "meteora_transaction_filter".to_string(),
        yellowstone_grpc_proto::geyser::SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![crate::meteora::constants::METEORA_PROGRAM_ID.to_string()],
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    // Determine optional X-Token header
    let x_token = if config.geyser_x_token.trim().is_empty() {
        None
    } else {
        Some(config.geyser_x_token.clone())
    };

    // Instantiate the Yellowstone gRPC client directly from endpoint URL (plain HTTP/2)
    let geyser_client = YellowstoneGrpcGeyserClient::new(
        config.geyser_url.clone(),
        x_token,
        Some(yellowstone_grpc_proto::geyser::CommitmentLevel::Processed),
        account_filters,
        tx_filters,
        Arc::new(RwLock::new(Default::default())),
    );

    // Start consuming updates
    geyser_client.consume(meteora).await?;
    Ok(())
}
