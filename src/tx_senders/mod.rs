use crate::config::{RpcConfig, RpcType};
use crate::tx_senders::jito::JitoTxSender;
use crate::tx_senders::solana_rpc::GenericRpc;
use crate::tx_senders::transaction::TransactionConfig;
use async_trait::async_trait;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use tracing::{info, warn};

pub mod bloxroute;
pub mod constants;
pub mod jito;
pub mod nextblock;
pub mod solana_rpc;
pub mod transaction;

#[derive(Debug, Clone)]
pub enum TxResult {
    Signature(Signature),
    BundleID(String),
}

impl Into<String> for TxResult {
    fn into(self) -> String {
        match self {
            TxResult::Signature(sig) => sig.to_string(),
            TxResult::BundleID(bundle_id) => bundle_id,
        }
    }
}

#[async_trait]
pub trait TxSender: Sync + Send {
    fn name(&self) -> String;

    /// Send a swap transaction targeting Meteora Dynamic AMM.
    /// `params` contains all accounts, `recent_blockhash` – latest hash.
    async fn send_meteora_swap(
        &self,
        params: &crate::meteora::types::MeteoraSwapParams,
        recent_blockhash: Hash,
    ) -> anyhow::Result<TxResult>;

    /// Get the current block height from the RPC node.
    async fn get_block_height(&self) -> anyhow::Result<u64>;
}

pub fn create_tx_sender(
    name: String,
    rpc_config: RpcConfig,
    tx_config: TransactionConfig,
    client: Client,
) -> Option<Arc<dyn TxSender>> {
    info!("create_tx_sender {:?}", rpc_config.rpc_type);
    match rpc_config.rpc_type {
        RpcType::SolanaRpc => Some(Arc::new(GenericRpc::new(
            name,
            rpc_config.url,
            tx_config,
            RpcType::SolanaRpc,
        ))),
        RpcType::Jito => Some(Arc::new(JitoTxSender::new(
            name,
            rpc_config.url.clone(),
            tx_config,
            client,
            Arc::new(RpcClient::new(rpc_config.url)),
        ))),
        RpcType::Bloxroute => {
            if rpc_config
                .auth
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            {
                Some(Arc::new(bloxroute::BloxrouteTxSender::new(
                    name,
                    rpc_config.url,
                    rpc_config.auth,
                    tx_config,
                    client,
                    Arc::new(RpcClient::new(
                        "https://api.mainnet-beta.solana.com".to_string(),
                    )),
                )))
            } else {
                warn!("Bloxroute sender '{name}' skipped – missing auth token");
                None
            }
        }
        RpcType::NextBlock => {
            if rpc_config
                .auth
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            {
                Some(Arc::new(nextblock::NextBlockTxSender::new(
                    name,
                    rpc_config.url,
                    rpc_config.auth,
                    tx_config,
                    client,
                    Arc::new(RpcClient::new(
                        "https://api.mainnet-beta.solana.com".to_string(),
                    )),
                )))
            } else {
                warn!("NextBlock sender '{name}' skipped – missing auth token");
                None
            }
        }
    }
}
