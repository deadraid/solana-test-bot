/// Re-plays a historical pool-initialization transaction
///
/// Usage: cargo run --release --bin inject_sim -- <TX_SIGNATURE_BASE58>

// Re-import project modules via explicit paths
#[path = "../bench.rs"]
mod bench;
#[path = "../config/mod.rs"]
mod config;
#[path = "../core/mod.rs"]
mod core;
#[path = "../geyser/mod.rs"]
mod geyser;
#[path = "../meteora/mod.rs"]
mod meteora;
#[path = "../tx_senders/mod.rs"]
mod tx_senders;

use std::str::FromStr;

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use bench::Bench;
use bincode::config::standard as bincode_standard_config;
use config::PingThingsArgs;
use meteora::controller::MeteoraController;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::message::v0::LoadedAddresses;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use solana_transaction_status::{
    EncodedTransaction, TransactionStatusMeta, UiLoadedAddresses, UiTransactionStatusMeta,
};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Default signature if none provided
    const DEFAULT_SIG: &str =
        "5QWwTAMs98vsPdYbeKbZvKfJQEbaxvB4XDP1EuNaDMXGyJ2Yu8pxnq21a9xmHuGgraYx8pted1qPA6jQQc2DX4ZH";

    let sig_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SIG.to_string());
    let signature = Signature::from_str(&sig_str).context("invalid base58 signature")?;

    // Fetch transaction from RPC
    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let tx: EncodedConfirmedTransactionWithStatusMeta = rpc
        .get_transaction(&signature, UiTransactionEncoding::Base64)
        .await
        .context("RPC get_transaction failed")?;

    // Decode transaction
    let encoded_tx = match &tx.transaction.transaction {
        EncodedTransaction::Binary(bin, _) => bin,
        _ => anyhow::bail!("transaction encoding is not binary"),
    };
    let tx_bytes = general_purpose::STANDARD
        .decode(encoded_tx)
        .context("base64 decode failed")?;
    let (versioned_tx, _): (VersionedTransaction, usize) =
        bincode::serde::decode_from_slice(&tx_bytes, bincode_standard_config())
            .context("bincode deserialize failed")?;

    let ui_meta: UiTransactionStatusMeta = tx
        .transaction
        .meta
        .context("missing meta in RPC response")?;

    // Construct TransactionStatusMeta from UI meta
    let meta = TransactionStatusMeta {
        status: ui_meta.err.map_or(Ok(()), Err),
        fee: ui_meta.fee,
        pre_balances: ui_meta.pre_balances,
        post_balances: ui_meta.post_balances,
        inner_instructions: None,
        log_messages: ui_meta.log_messages.into(),
        pre_token_balances: None,
        post_token_balances: None,
        rewards: ui_meta.rewards.into(),
        loaded_addresses: {
            let opt_ui_loaded_addresses: Option<UiLoadedAddresses> =
                ui_meta.loaded_addresses.into();
            match opt_ui_loaded_addresses {
                Some(ui_loaded) => LoadedAddresses {
                    writable: ui_loaded
                        .writable
                        .into_iter()
                        .map(|s| Pubkey::from_str(&s))
                        .collect::<Result<Vec<_>, _>>()?,
                    readonly: ui_loaded
                        .readonly
                        .into_iter()
                        .map(|s| Pubkey::from_str(&s))
                        .collect::<Result<Vec<_>, _>>()?,
                },
                None => LoadedAddresses::default(),
            }
        },
        return_data: None,
        compute_units_consumed: ui_meta.compute_units_consumed.into(),
    };

    // Run through Meteora controller
    let mut config = PingThingsArgs::new();
    config.simulate = true; // Override: inject_sim ALWAYS simulates
    let bench = Bench::new(config.clone());
    let mut controller = MeteoraController::new(config, bench);

    controller
        .transaction_handler(signature, versioned_tx, meta, false, 0)
        .await?;

    Ok(())
}
