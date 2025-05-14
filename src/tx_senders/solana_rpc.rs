use crate::config::RpcType;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{TxResult, TxSender};
use async_trait::async_trait;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::hash::Hash;
use solana_transaction_status::UiTransactionEncoding;
use std::sync::Arc;

#[derive(Clone)]
pub struct GenericRpc {
    pub name: String,
    pub http_rpc: Arc<RpcClient>,
    tx_config: TransactionConfig,
    rpc_type: RpcType,
}

#[derive(Serialize, Debug)]
pub struct TxMetrics {
    pub rpc_name: String,
    pub signature: String,
    pub index: u32,
    pub success: bool,
    pub slot_sent: u64,
    pub slot_landed: Option<u64>,
    pub slot_latency: Option<u64>,
    pub elapsed: Option<u64>, // in milliseconds
}

impl GenericRpc {
    pub fn new(name: String, url: String, config: TransactionConfig, rpc_type: RpcType) -> Self {
        let http_rpc = Arc::new(RpcClient::new(url));
        GenericRpc {
            name,
            http_rpc,
            tx_config: config,
            rpc_type,
        }
    }
}

#[async_trait]
impl TxSender for GenericRpc {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_meteora_swap(
        &self,
        params: &crate::meteora::types::MeteoraSwapParams,
        recent_blockhash: Hash,
    ) -> anyhow::Result<TxResult> {
        let tx = crate::tx_senders::transaction::build_meteora_swap_tx(
            &self.tx_config,
            &self.rpc_type,
            recent_blockhash,
            params,
        );
        let sig = self
            .http_rpc
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: None,
                    encoding: Some(UiTransactionEncoding::Base64),
                    max_retries: None,
                    min_context_slot: None,
                },
            )
            .await?;
        Ok(TxResult::Signature(sig))
    }

    async fn get_block_height(&self) -> anyhow::Result<u64> {
        Ok(self.http_rpc.get_block_height().await?)
    }
}
