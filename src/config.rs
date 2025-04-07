use key_utils::Secp256k1PublicKey;
use key_utils::Secp256k1SecretKey;
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Clone, Deserialize, Debug)]
pub struct PlebLotteryMiningServerConfig {
    pub listening_port: u16,
    pub pub_key: Secp256k1PublicKey,
    pub priv_key: Secp256k1SecretKey,
    pub cert_validity: u64,
    pub inactivity_limit: u64,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PlebLotteryTemplateDistributionClientConfig {
    pub server_addr: SocketAddr,
    pub auth_pk: Option<Secp256k1PublicKey>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PleblotteryConfig {
    pub mining_server_config: PlebLotteryMiningServerConfig,
    pub template_distribution_config: PlebLotteryTemplateDistributionClientConfig,
}

impl PleblotteryConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }
}
