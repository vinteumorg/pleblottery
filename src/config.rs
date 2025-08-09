use bitcoin::Address;
use serde::{Deserialize, Deserializer};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use sv2_services::client::service::config::Sv2ClientServiceConfig;
use sv2_services::client::service::config::Sv2ClientServiceTemplateDistributionConfig;
use sv2_services::key_utils::Secp256k1PublicKey;
use sv2_services::key_utils::Secp256k1SecretKey;
use sv2_services::server::service::config::Sv2ServerServiceConfig;
use sv2_services::server::service::config::Sv2ServerServiceMiningConfig;
use sv2_services::server::service::config::Sv2ServerTcpConfig;
#[derive(Clone, Debug)]
pub struct PlebLotteryMiningServerConfig {
    pub listening_port: u16,
    pub pub_key: Secp256k1PublicKey,
    pub priv_key: Secp256k1SecretKey,
    pub cert_validity: u64,
    pub inactivity_limit: u64,
    pub coinbase_output_script: bitcoin::ScriptBuf,
    pub coinbase_tag: String,
    pub share_batch_size: usize,
    pub expected_shares_per_minute: f32,
}

impl<'de> Deserialize<'de> for PlebLotteryMiningServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            listening_port: u16,
            pub_key: Secp256k1PublicKey,
            priv_key: Secp256k1SecretKey,
            cert_validity: u64,
            inactivity_limit: u64,
            coinbase_output_address: String,
            coinbase_tag: String,
            share_batch_size: usize,
            expected_shares_per_minute: f32,
        }
        let helper = Helper::deserialize(deserializer).map_err(|e| {
            serde::de::Error::custom(format!("Failed to deserialize mining server config: {e}"))
        })?;

        if helper.coinbase_tag.len() > 8 {
            return Err(serde::de::Error::custom(
                "coinbase_tag must have at most 8 characters",
            ));
        }

        let address = Address::from_str(&helper.coinbase_output_address)
            .map_err(|e| serde::de::Error::custom(format!("Invalid coinbase output address: {e}")))?
            .assume_checked();
        Ok(PlebLotteryMiningServerConfig {
            listening_port: helper.listening_port,
            pub_key: helper.pub_key,
            priv_key: helper.priv_key,
            cert_validity: helper.cert_validity,
            inactivity_limit: helper.inactivity_limit,
            coinbase_output_script: address.script_pubkey(),
            coinbase_tag: helper.coinbase_tag,
            share_batch_size: helper.share_batch_size,
            expected_shares_per_minute: helper.expected_shares_per_minute,
        })
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct PlebLotteryTemplateDistributionClientConfig {
    pub server_addr: SocketAddr,
    pub auth_pk: Option<Secp256k1PublicKey>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PlebLotteryWebConfig {
    pub listening_port: u16,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PleblotteryConfig {
    pub mining_server_config: PlebLotteryMiningServerConfig,
    pub template_distribution_config: PlebLotteryTemplateDistributionClientConfig,
    pub web_config: PlebLotteryWebConfig,
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
                setup_connection_flags: 0,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pub_key() -> Secp256k1PublicKey {
        Secp256k1PublicKey::from_str("9bDuixKmZqAJnrmP746n8zU1wyAQRrus7th9dxnkPg6RzQvCnan").unwrap()
    }

    fn dummy_priv_key() -> Secp256k1SecretKey {
        Secp256k1SecretKey::from_str("zmBEmPhqo3A92FkiLVvyCz6htc3e53ph3ZbD4ASqGaLjwnFLi").unwrap()
    }

    fn make_config(address: &str) -> PlebLotteryMiningServerConfig {
        let address: Address = Address::from_str(address).unwrap().assume_checked();
        PlebLotteryMiningServerConfig {
            listening_port: 8332,
            pub_key: dummy_pub_key(),
            priv_key: dummy_priv_key(),
            cert_validity: 3600,
            inactivity_limit: 300,
            coinbase_output_script: address.script_pubkey(),
            coinbase_tag: "test".to_string(),
            share_batch_size: 10,
            expected_shares_per_minute: 1.0,
        }
    }

    #[test]
    fn test_coinbase_script_various_types() {
        let cases = vec![
            // (address, expected script prefix, description)
            (
                "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", // P2PKH
                "76a9",
                "Mainnet P2PKH",
            ),
            (
                "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy", // P2SH
                "a914",
                "Mainnet P2SH",
            ),
            (
                "bc1qryhgpmfv03qjhhp2dj8nw8g4ewg08jzmgy3cyx", // P2WPKH
                "0014",
                "Mainnet P2WPKH",
            ),
            (
                "bc1p2m7q0yn78rjqh200dz0kut5xcxdnfxk4wcsau7zydnrv9ns875eq37vmkz", // Taproot
                "5120",
                "Mainnet Taproot",
            ),
            (
                "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn", // P2PKH
                "76a9",
                "Testnet P2PKH",
            ),
            (
                "2N2JD6wb56AfK4tfmM6PwdVmoYk2dCKf4Br", // P2SH
                "a914",
                "Testnet P2SH",
            ),
            (
                "tb1qw8rnkgnk7s48h6w5c0mg7we7gvzykeyp2sze82", // P2WPKH
                "0014",
                "Testnet P2WPKH",
            ),
            (
                "tb1pktwvz28qttg8k6r9wkzrp75lek4tnl6qn9wezfz3l8nhy57q886qf9azpd", // Taproot
                "5120",
                "Testnet Taproot",
            ),
            (
                "bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl", // Regtest P2WPKH
                "0014",
                "Regtest P2WPKH",
            ),
        ];

        for (address, expected_prefix, description) in cases {
            let config: PlebLotteryMiningServerConfig = make_config(address);
            let script = config.coinbase_output_script.clone();
            let hex = script.to_hex_string();
            assert!(
                hex.starts_with(expected_prefix),
                "Failed: {}, script: {}",
                description,
                hex
            );
        }
    }
    #[test]
    fn test_coinbase_script_invalid_address_error() {
        let result = std::panic::catch_unwind(|| make_config("this_is_not_a_valid_address"));
        assert!(result.is_err(), "Expected panic for invalid address");
    }
}
