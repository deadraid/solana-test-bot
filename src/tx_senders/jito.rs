use crate::config::RpcType;
use crate::meteora::types::MeteoraSwapParams;
use crate::tx_senders::transaction::{build_meteora_swap_tx, TransactionConfig};
use crate::tx_senders::{TxResult, TxSender};

use anyhow::Context;
use async_trait::async_trait;
use bincode::config;
use bincode::serde as bincode_serde;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use std::sync::Arc;
use tracing::info;

pub struct JitoTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
    /// For getting block height/checking status
    rpc_client: Arc<RpcClient>,
}

impl JitoTxSender {
    pub fn new(
        name: String,
        url: String,
        tx_config: TransactionConfig,
        client: Client,
        rpc_client: Arc<RpcClient>,
    ) -> Self {
        Self {
            url,
            name,
            client,
            tx_config,
            rpc_client,
        }
    }
}

#[async_trait]
impl TxSender for JitoTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Send a single swap transaction as a raw-bundle to the block-engine.
    async fn send_meteora_swap(
        &self,
        params: &MeteoraSwapParams,
        recent_blockhash: Hash,
    ) -> anyhow::Result<TxResult> {
        // 1. Build VersionedTransaction
        let tx = build_meteora_swap_tx(&self.tx_config, &RpcType::Jito, recent_blockhash, params);

        // 2. Serialize to raw bytes (bincode) — this is exactly what block-engine expects.
        let config = config::standard();
        let tx_bytes = bincode_serde::encode_to_vec(&tx, config).context("cannot serialize tx")?;

        // 3. Send as `application/octet-stream`
        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/octet-stream")
            .body(tx_bytes)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("bundle submit failed: {}", body));
        }

        // block-engine returns a BundleID string in JSON (usually just `"uuid"`).
        let bundle_id = body.trim_matches('"').to_string();
        info!(target: "meteora", "raw-bundle accepted: {bundle_id}");
        Ok(TxResult::BundleID(bundle_id))
    }

    /// For logs/metrics, a regular RPC client can be called
    async fn get_block_height(&self) -> anyhow::Result<u64> {
        Ok(self.rpc_client.get_block_height().await?)
    }
}
