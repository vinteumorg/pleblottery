use serde::Deserialize;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use tower_stratum::client::service::config::Sv2ClientServiceConfig;
use tower_stratum::client::service::config::Sv2ClientServiceTemplateDistributionConfig;
use tower_stratum::key_utils::Secp256k1PublicKey;
use tower_stratum::key_utils::Secp256k1SecretKey;
use tower_stratum::server::service::config::Sv2ServerServiceConfig;
use tower_stratum::server::service::config::Sv2ServerServiceMiningConfig;
use tower_stratum::server::service::config::Sv2ServerTcpConfig;
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
        let contents = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let config: Self = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
        Ok(config)
    }
}

impl From<PlebLotteryMiningServerConfig> for Sv2ServerServiceConfig {
    fn from(config: PlebLotteryMiningServerConfig) -> Self {
        Sv2ServerServiceConfig {
            min_supported_version: 2,
            max_supported_version: 2,
            inactivity_limit: config.inactivity_limit,
            tcp_config: Sv2ServerTcpConfig {
                listen_address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    config.listening_port,
                ),
                pub_key: config.pub_key,
                priv_key: config.priv_key,
                cert_validity: config.cert_validity,
            },
            mining_config: Some(Sv2ServerServiceMiningConfig {
                supported_flags: 0b0101, // standard jobs are supported, rolling version bits is supported
            }),
            job_declaration_config: None,
            template_distribution_config: None,
        }
    }
}

impl From<PlebLotteryTemplateDistributionClientConfig> for Sv2ClientServiceConfig {
    fn from(config: PlebLotteryTemplateDistributionClientConfig) -> Self {
        Sv2ClientServiceConfig {
            min_supported_version: 2,
            max_supported_version: 2,
            endpoint_host: None,
            endpoint_port: None,
            vendor: None,
            hardware_version: None,
            device_id: None,
            firmware: None,
            mining_config: None,
            job_declaration_config: None,
            template_distribution_config: Some(Sv2ClientServiceTemplateDistributionConfig {
                server_addr: config.server_addr,
                auth_pk: config.auth_pk,
                coinbase_output_constraints: (1, 1), // todo: fix this
            }),
        }
    }
}
