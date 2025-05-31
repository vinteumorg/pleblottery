use anyhow::Ok;
use bitcoin::address::NetworkChecked;
use bitcoin::{network, Address};
use serde::Deserialize;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
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

    /// The Bitcoin address to which the coinbase output will be sent.
    ///
    /// This should be a valid address string for the selected network (see `mining_network`).
    ///
    /// Supported address types:
    ///   - P2PKH (e.g., starts with '1' for mainnet, 'm' or 'n' for testnet)
    ///   - P2SH  (e.g., starts with '3' for mainnet, '2' for testnet)
    ///   - Bech32 SegWit (P2WPKH, P2WSH, Taproot):
    ///       - Mainnet: starts with 'bc1...'
    ///       - Testnet: starts with 'tb1...'
    ///       - Regtest: starts with 'bcrt1...'
    ///
    /// Example values:
    ///   "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa" (mainnet P2PKH)
    ///   "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy" (mainnet P2SH)
    ///   "bc1qryhgpmfv03qjhhp2dj8nw8g4ewg08jzmgy3cyx" (mainnet P2WPKH)
    ///   "tb1qw8rnkgnk7s48h6w5c0mg7we7gvzykeyp2sze82" (testnet P2WPKH)
    pub coinbase_output_address: String,
    /// The Bitcoin network to use for mining and address validation.
    ///
    /// Supported values: "bitcoin", "testnet", "regtest".
    /// This must match the network of the `coinbase_output_address`.
    pub mining_network: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct PlebLotteryTemplateDistributionClientConfig {
    pub server_addr: SocketAddr,
    pub auth_pk: Option<Secp256k1PublicKey>,
}

impl PlebLotteryMiningServerConfig {
    pub fn coinbase_output_script(&self) -> anyhow::Result<bitcoin::ScriptBuf> {
        let network = network::Network::from_str(self.mining_network.as_str())
            .map_err(|e| anyhow::anyhow!("Invalid mining network: {}", e))?;

        let address: Address<NetworkChecked> = Address::from_str(&self.coinbase_output_address)
            .map_err(|e| anyhow::anyhow!("Failed to parse address: {}", e))?
            .require_network(network)
            .map_err(|_| anyhow::anyhow!("Address network mismatch"))?;

        Ok(address.script_pubkey())
    }
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

    fn make_config(address: &str, network: &str) -> PlebLotteryMiningServerConfig {
        PlebLotteryMiningServerConfig {
            listening_port: 8332,
            pub_key: dummy_pub_key(),
            priv_key: dummy_priv_key(),
            cert_validity: 3600,
            inactivity_limit: 300,
            coinbase_output_address: address.to_string(),
            mining_network: network.to_string(),
        }
    }

    #[test]
    fn test_coinbase_script_various_types() {
        let cases = vec![
            // (address, network, expected script prefix, description)
            (
                "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", // P2PKH
                "bitcoin",
                "76a9",
                "Mainnet P2PKH",
            ),
            (
                "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy", // P2SH
                "bitcoin",
                "a914",
                "Mainnet P2SH",
            ),
            (
                "bc1qryhgpmfv03qjhhp2dj8nw8g4ewg08jzmgy3cyx", // P2WPKH
                "bitcoin",
                "0014",
                "Mainnet P2WPKH",
            ),
            (
                "bc1p2m7q0yn78rjqh200dz0kut5xcxdnfxk4wcsau7zydnrv9ns875eq37vmkz", // Taproot
                "bitcoin",
                "5120",
                "Mainnet Taproot",
            ),
            (
                "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn", // P2PKH
                "testnet",
                "76a9",
                "Testnet P2PKH",
            ),
            (
                "2N2JD6wb56AfK4tfmM6PwdVmoYk2dCKf4Br", // P2SH
                "testnet",
                "a914",
                "Testnet P2SH",
            ),
            (
                "tb1qw8rnkgnk7s48h6w5c0mg7we7gvzykeyp2sze82", // P2WPKH
                "testnet",
                "0014",
                "Testnet P2WPKH",
            ),
            (
                "tb1pktwvz28qttg8k6r9wkzrp75lek4tnl6qn9wezfz3l8nhy57q886qf9azpd", // Taproot
                "testnet",
                "5120",
                "Testnet Taproot",
            ),
            (
                "bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl", // Regtest P2WPKH
                "regtest",
                "0014",
                "Regtest P2WPKH",
            ),
        ];

        for (address, network, expected_prefix, description) in cases {
            let config = make_config(address, network);
            let script = config.coinbase_output_script().expect(description);
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
    fn test_coinbase_script_network_mismatch_error() {
        let config = make_config(
            "bc1qypgev2h0x0p0w7efw6pkd8xuv85ccxj7za4ljp", // Mainnet P2WPKH
            "testnet",
        );
        let err = config.coinbase_output_script().unwrap_err();
        assert!(
            format!("{err}").contains("Address network mismatch"),
            "Unexpected error message: {err}"
        );
    }

    #[test]
    fn test_coinbase_script_invalid_address_error() {
        let config = make_config("this_is_not_a_valid_address", "testnet");
        let err = config.coinbase_output_script().unwrap_err();
        assert!(
            format!("{err}").contains("Failed to parse address"),
            "Unexpected error message: {err}"
        );
    }

    #[test]
    fn test_coinbase_script_invalid_network_error() {
        let config = make_config("tb1qw8rnkgnk7s48h6w5c0mg7we7gvzykeyp2sze82", "banana");
        let err = config.coinbase_output_script().unwrap_err();
        assert!(
            format!("{err}").contains("Invalid mining network"),
            "Unexpected error message: {err}"
        );
    }
}
