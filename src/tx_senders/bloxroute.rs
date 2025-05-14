use crate::config::RpcType;
use crate::tx_senders::transaction::{build_meteora_swap_tx, TransactionConfig};
use crate::tx_senders::{TxResult, TxSender};

use anyhow::Context;
use async_trait::async_trait;
use base64::{self, engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use bincode::{config, serde as bincode_serde};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Signature;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct BloxrouteTxSender {
    url: String,
    name: String,
    auth_header: Option<String>,
    client: Client,
    tx_config: TransactionConfig,
    rpc_client: Arc<RpcClient>,
}

impl BloxrouteTxSender {
    pub fn new(
        name: String,
        url: String,
        auth_header: Option<String>,
        tx_config: TransactionConfig,
        client: Client,
        rpc_client: Arc<RpcClient>,
    ) -> Self {
        Self {
            url,
            name,
            auth_header,
            client,
            tx_config,
            rpc_client,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SubmitResponse {
    signature: String,
}

#[async_trait]
impl TxSender for BloxrouteTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_meteora_swap(
        &self,
        params: &crate::meteora::types::MeteoraSwapParams,
        recent_blockhash: Hash,
    ) -> anyhow::Result<TxResult> {
        // Build VersionedTransaction
        let tx = build_meteora_swap_tx(
            &self.tx_config,
            &RpcType::SolanaRpc,
            recent_blockhash,
            params,
        );

        // Serialize to raw bytes and then base64
        let cfg = config::standard();
        let tx_bytes = bincode_serde::encode_to_vec(&tx, cfg).context("cannot serialize tx")?;
        let tx_base64 = BASE64_STD.encode(tx_bytes);

        // Prepare JSON body and serialize manually
        let body = json!({
            "transaction": { "content": tx_base64 },
            "skipPreFlight": true
        });
        let body_str = serde_json::to_string(&body)?;

        // Build request
        let mut req = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body_str);
        if let Some(auth) = &self.auth_header {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("bloxroute submit failed: {}", text));
        }

        // Try parse json {"signature":"..."}
        let sig_resp: Result<SubmitResponse, _> = serde_json::from_str(&text);
        let signature = match sig_resp {
            Ok(r) => r.signature,
            Err(_) => text.trim_matches('"').to_string(),
        };

        info!(target: "meteora", "bloxroute tx accepted: {signature}");
        let sig = Signature::from_str(&signature)?;
        Ok(TxResult::Signature(sig))
    }

    async fn get_block_height(&self) -> anyhow::Result<u64> {
        Ok(self.rpc_client.get_block_height().await?)
    }
}
