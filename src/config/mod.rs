use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fs;

#[derive(Clone, Deserialize)]
pub struct PingThingsArgs {
    // rpc_name -> rpc_url
    pub rpc: HashMap<String, RpcConfig>,
    pub http_rpc: String,
    pub ws_rpc: String,
    pub geyser_url: String,
    #[allow(dead_code)]
    pub geyser_x_token: String,
    pub private_key: String,
    pub compute_unit_price: u64,
    pub compute_unit_limit: u32,
    pub tip: f64,
    pub buy_amount: f64,
    pub min_amount_out: f64,
    #[serde(default)]
    pub simulate: bool,
}

// Custom Debug implementation that redacts private key
impl fmt::Debug for PingThingsArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PingThingsArgs")
            .field("rpc", &self.rpc)
            .field("http_rpc", &self.http_rpc)
            .field("ws_rpc", &self.ws_rpc)
            .field("geyser_url", &self.geyser_url)
            .field("geyser_x_token", &"[REDACTED]")
            .field("private_key", &"[REDACTED]")
            .field("compute_unit_price", &self.compute_unit_price)
            .field("compute_unit_limit", &self.compute_unit_limit)
            .field("tip", &self.tip)
            .field("buy_amount", &self.buy_amount)
            .field("min_amount_out", &self.min_amount_out)
            .field("simulate", &self.simulate)
            .finish()
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "lowercase")] // Allows lowercase matching for variants
pub enum RpcType {
    #[default]
    SolanaRpc,
    Jito,
    /// bloXroute Trader API
    Bloxroute,
    /// NextBlock transaction API
    NextBlock,
}
#[derive(Clone, Debug, Deserialize)]
pub struct RpcConfig {
    pub url: String,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub rpc_type: RpcType,
}

impl PingThingsArgs {
    pub fn new() -> Self {
        let config_yaml = fs::read_to_string("./config.yaml").expect("cannot find config file");
        serde_yaml::from_str::<PingThingsArgs>(&config_yaml).expect("invalid config file")
    }
}
