use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sv1Config {
    pub listen_host: String,
    pub listen_port: u16,
    pub bitcoin_rpc_host: String,
    pub bitcoin_rpc_port: u16,
    pub bitcoin_rpc_user: String,
    pub bitcoin_rpc_pass: String,
    pub bitcoin_network: String,
    pub getblocktemplate_interval: f32,
    pub solo_miner_signature: String,
    pub solo_miner_address: String,
}
