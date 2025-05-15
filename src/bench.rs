use crate::config::{PingThingsArgs, RpcType};
use crate::meteora::types::MeteoraSwapParams;
use crate::tx_senders::{
    create_tx_sender,
    solana_rpc::TxMetrics,
    transaction::{build_meteora_swap_tx, TransactionConfig},
    TxSender,
};

use anyhow::{Context, Result};
use base64::{self, engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use bincode;
use log::{debug, error, info, warn};
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_transaction_status::UiTransactionEncoding;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Holds shared state for broadcasting (or simulating) swap transactions.
#[derive(Clone)]
pub struct Bench {
    /// Full CLI / YAML config.
    pub config: PingThingsArgs,
    /// Pre-built static tx parameters (keypair, cu-limit, etc.).
    pub tx_config: TransactionConfig,
    /// Channel for optional external metrics (not used here but kept for compatibility).
    #[allow(dead_code)]
    pub tx_subscribe_sender: mpsc::Sender<TxMetrics>,
    /// List of RPC / Jito senders.
    pub rpcs: Vec<Arc<dyn TxSender>>,
    /// Shared Reqwest client.
    #[allow(dead_code)]
    pub client: Client,
}

impl Bench {
    /// Create a new `Bench` from global `PingThingsArgs`.
    pub fn new(config: PingThingsArgs) -> Self {
        let (tx_subscribe_sender, _rx) = mpsc::channel(100);

        // Build once – can be reused for every tx
        let tx_config: TransactionConfig = config.clone().into();
        let client = Client::new();

        // Convert every entry in `rpc:` map into a concrete sender
        let rpcs = config
            .rpc
            .clone()
            .into_iter()
            .filter_map(|(name, rpc)| {
                create_tx_sender(name, rpc, tx_config.clone(), client.clone())
            })
            .collect::<Vec<_>>();

        Self {
            config,
            tx_config,
            tx_subscribe_sender,
            rpcs,
            client,
        }
    }

    /// Either **simulate** or **broadcast** a single swap using a given sender.
    async fn send_or_simulate(
        &self,
        rpc_sender: Arc<dyn TxSender>,
        recent_blockhash: Hash,
        params: MeteoraSwapParams,
    ) -> Result<()> {
        // -------- Simulation mode --------
        if self.config.simulate {
            let rpc_client = RpcClient::new(self.config.http_rpc.clone());
            let latest_blockhash = rpc_client
                .get_latest_blockhash()
                .await
                .context("failed to fetch recent blockhash for simulation")?;

            let versioned_tx = build_meteora_swap_tx(
                &self.tx_config,
                &RpcType::SolanaRpc, // RPC type for simulation purposes
                latest_blockhash,
                &params,
            );

            let sim_cfg = RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                commitment: Some(CommitmentConfig::processed()),
                encoding: Some(UiTransactionEncoding::Base64),
                accounts: None,
                min_context_slot: None,
                inner_instructions: false,
            };
            let sim_res = rpc_client
                .simulate_transaction_with_config(&versioned_tx, sim_cfg)
                .await
                .context("simulation RPC failed")?;

            if let Some(err_details) = &sim_res.value.err {
                info!(
                    "[SIM] {} → Simulation FAILED. Error: {:?}. Potential issues: insufficient funds, incorrect accounts, smart contract error, high slippage. Consumed CU: {}",
                    rpc_sender.name(),
                    err_details,
                    sim_res.value.units_consumed.unwrap_or_default()
                );
            } else {
                info!(
                    "[SIM] {} → Simulation SUCCESSFUL. Consumed CU: {}",
                    rpc_sender.name(),
                    sim_res.value.units_consumed.unwrap_or_default()
                );
            }

            if let Some(logs) = sim_res.value.logs {
                for l in logs {
                    debug!("[SIM_LOG] {}", l);
                }
            }

            let tx_bytes =
                bincode::serde::encode_to_vec(&versioned_tx, bincode::config::standard()).unwrap();
            let tx_base64 = BASE64_STD.encode(tx_bytes);
            debug!("[SIM_TX_BASE64] {}", tx_base64);

            debug!(
                "[SIM_ACCOUNTS] total={}",
                versioned_tx.message.static_account_keys().len()
            );
            for (idx, pk) in versioned_tx
                .message
                .static_account_keys()
                .iter()
                .enumerate()
            {
                debug!("[SIM_ACCOUNT_{}] {}", idx, pk);
            }

            return Ok(());
        }

        // -------- Real broadcast --------
        // Capture current block height before submitting the transaction so we can
        // later compute how many blocks it took to land (≈ latency in blocks).
        let slot_sent = rpc_sender.get_block_height().await.ok();

        let started = Instant::now();
        let tx_result = rpc_sender
            .send_meteora_swap(&params, recent_blockhash)
            .await?;
        info!(
            "Swap via {} took {} ms – {:?}",
            rpc_sender.name(),
            started.elapsed().as_millis(),
            tx_result
        );

        // Fetch latest block height after broadcast and compute Δ in slots, if possible.
        match rpc_sender.get_block_height().await {
            Ok(height) => {
                if let Some(sent) = slot_sent {
                    let delta = height.saturating_sub(sent);
                    info!(
                        "{} confirmed near block {}, Δ={} slots",
                        rpc_sender.name(),
                        height,
                        delta
                    );
                } else {
                    info!("{} confirmed near block {}", rpc_sender.name(), height);
                }
            }
            Err(e) => {
                warn!(
                    "failed to fetch block-height via {}: {}",
                    rpc_sender.name(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Public helper the controller calls after it builds `MeteoraSwapParams`.
    pub async fn send_buy_tx_meteora(&self, recent_blockhash: Hash, params: MeteoraSwapParams) {
        let mut tasks = Vec::new();

        for rpc in &self.rpcs {
            let sender = rpc.clone();
            let rb = recent_blockhash;
            let p = params.clone();
            let bench_ref = self.clone();

            let handle = tokio::spawn(async move {
                if let Err(e) = bench_ref.send_or_simulate(sender, rb, p).await {
                    error!("swap send failed: {:?}", e);
                }
            });
            tasks.push(handle);
        }

        for h in tasks {
            let _ = h.await;
        }

        if self.config.simulate {
            info!("All simulations finished");
        } else {
            info!("All swap broadcasts finished");
        }
    }
}
