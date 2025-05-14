use async_trait::async_trait;
use futures::StreamExt;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::tonic::transport::ClientTlsConfig;
use yellowstone_grpc_proto::{
    convert_from::{create_tx_meta, create_tx_versioned},
    geyser::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
        SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
    },
};

#[derive(Debug)]
pub struct YellowstoneGrpcGeyserClient {
    pub endpoint: String,
    pub x_token: Option<String>,
    pub commitment: Option<CommitmentLevel>,
    pub account_filters: HashMap<String, SubscribeRequestFilterAccounts>,
    pub transaction_filters: HashMap<String, SubscribeRequestFilterTransactions>,
    pub account_deletions_tracked: Arc<RwLock<HashSet<Pubkey>>>,
}

impl YellowstoneGrpcGeyserClient {
    #[allow(dead_code)]
    pub fn new(
        endpoint: String,
        x_token: Option<String>,
        commitment: Option<CommitmentLevel>,
        account_filters: HashMap<String, SubscribeRequestFilterAccounts>,
        transaction_filters: HashMap<String, SubscribeRequestFilterTransactions>,
        account_deletions_tracked: Arc<RwLock<HashSet<Pubkey>>>,
    ) -> Self {
        YellowstoneGrpcGeyserClient {
            endpoint,
            x_token,
            commitment,
            account_filters,
            transaction_filters,
            account_deletions_tracked,
        }
    }
}

pub type GeyserResult<T> = Result<T, Error>;

#[async_trait]
pub trait YellowstoneGrpcGeyser: Send + Sync {
    #[allow(dead_code)]
    async fn consume(&self, meteora_controller: MeteoraController) -> GeyserResult<()>;
}

use crate::meteora::controller::MeteoraController;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Custom error: {0}")]
    Custom(String),
}

#[async_trait]
impl YellowstoneGrpcGeyser for YellowstoneGrpcGeyserClient {
    async fn consume(&self, mut meteora_controller: MeteoraController) -> GeyserResult<()> {
        let endpoint = self.endpoint.clone();
        let x_token = self.x_token.clone();
        let commitment = self.commitment;
        let account_filters = self.account_filters.clone();
        let transaction_filters = self.transaction_filters.clone();
        let _account_deletions_tracked = self.account_deletions_tracked.clone();

        let mut geyser_client = GeyserGrpcClient::build_from_shared(endpoint.clone())
            .map_err(|err| Error::Custom(err.to_string()))?
            .x_token(x_token.clone())
            .map_err(|err| Error::Custom(err.to_string()))?
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(15))
            .tls_config(ClientTlsConfig::new().with_enabled_roots())
            .map_err(|err| Error::Custom(err.to_string()))?
            .connect()
            .await
            .map_err(|err| Error::Custom(err.to_string()))?;

        info!(target: "geyser", "✅ Connected to Geyser endpoint {endpoint}");

        let _ = tokio::spawn(async move {
            let subscribe_request = SubscribeRequest {
                slots: HashMap::new(),
                accounts: account_filters,
                transactions: transaction_filters,
                transactions_status: HashMap::new(),
                entry: HashMap::new(),
                blocks: HashMap::new(),
                blocks_meta: HashMap::new(),
                commitment: commitment.map(|x| x as i32),
                accounts_data_slice: vec![],
                ping: None,
                from_slot: None,
            };

            loop {
                tokio::select! {
                    result = geyser_client.subscribe_with_request(Some(subscribe_request.clone())) => {
                        match result {
                            Ok((_subscribe_tx, mut stream)) => {
                                while let Some(message) = stream.next().await {
                                    match message {
                                        Ok(msg) => match msg.update_oneof {
                                            Some(UpdateOneof::Transaction(transaction_update)) => {
                                                // Lightweight heartbeat for debugging
                                                tracing::debug!(target: "geyser", "slot {}", transaction_update.slot);

                                                let _start_time = std::time::Instant::now();

                                                if let Some(transaction_info) =
                                                    transaction_update.transaction
                                                {
                                                    let Ok(signature) =
                                                        Signature::try_from(transaction_info.signature)
                                                    else {
                                                        continue;
                                                    };
                                                    let Some(yellowstone_transaction) =
                                                        transaction_info.transaction
                                                    else {
                                                        continue;
                                                    };
                                                    let Some(yellowstone_tx_meta) = transaction_info.meta
                                                    else {
                                                        continue;
                                                    };
                                                    let Ok(versioned_transaction) =
                                                        create_tx_versioned(yellowstone_transaction)
                                                    else {
                                                        continue;
                                                    };
                                                    let meta_original = match create_tx_meta(
                                                        yellowstone_tx_meta,
                                                    ) {
                                                        Ok(meta) => meta,
                                                        Err(err) => {
                                                            log::error!(
                                                                "Failed to create transaction meta: {:?}",
                                                                err
                                                            );
                                                            continue;
                                                        }
                                                    };
                                                    // info!("signature {:?}", signature);
                                                     let  _ = meteora_controller.transaction_handler(
                                                        signature,
                                                        versioned_transaction,
                                                        meta_original,
                                                        transaction_info.is_vote,
                                                        transaction_update.slot
                                                     ).await;
                                                } else {
                                                    log::error!("No transaction info in `UpdateOneof::Transaction` at slot {}", transaction_update.slot);
                                                }
                                            }

                                            _ => {}
                                        },
                                        Err(error) => {
                                            log::error!("Geyser stream error: {error:?}");
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to subscribe: {:?}", e);
                            }
                        }
                    }
                }
            }
        }).await;

        Ok(())
    }
}
