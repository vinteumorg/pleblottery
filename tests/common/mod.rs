use once_cell::sync::Lazy;
use std::{
    collections::HashSet,
    net::{SocketAddr, TcpListener},
    str::FromStr,
    sync::Mutex,
};
use sv2_cpu_miner::config::Sv2CpuMinerConfig;

use bitcoin::Address;
use pleblottery::config::{
    PlebLotteryMiningServerConfig, PlebLotteryTemplateDistributionClientConfig,
};
use pleblottery::config::{PlebLotteryWebConfig, PleblotteryConfig};

// prevents get_available_port from ever returning the same port twice
static UNIQUE_PORTS: Lazy<Mutex<HashSet<u16>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn get_available_address() -> SocketAddr {
    let port = get_available_port();
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn get_available_port() -> u16 {
    let mut unique_ports = UNIQUE_PORTS.lock().unwrap();

    loop {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        if !unique_ports.contains(&port) {
            unique_ports.insert(port);
            return port;
        }
    }
}

pub fn load_config() -> PleblotteryConfig {
    let mining_server_available_addr = get_available_address();
    let web_server_available_addr = get_available_address();

    PleblotteryConfig {
        mining_server_config: PlebLotteryMiningServerConfig {
            listening_port: mining_server_available_addr.port(),
            pub_key: "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
                .parse()
                .expect("Invalid public key"),
            priv_key: "mkDLTBBRxdBv998612qipDYoTK3YUrqLe8uWw7gu3iXbSrn2n"
                .parse()
                .expect("Invalid private key"),
            cert_validity: 3600,
            inactivity_limit: 3600,
            coinbase_output_script: Address::from_str(
                "bcrt1q2nfxmhd4n3c8834pj72xagvyr9gl57n5r94fsl",
            )
            .unwrap()
            .assume_checked()
            .script_pubkey(),
            coinbase_tag: "pleblottery".to_string(),
            share_batch_size: 10,
            expected_shares_per_minute: 1.0,
        },
        template_distribution_config: PlebLotteryTemplateDistributionClientConfig {
            server_addr: "127.0.0.1:8442".parse().expect("Invalid server address"),
            auth_pk: None,
        },
        web_config: PlebLotteryWebConfig {
            listening_port: web_server_available_addr.port(),
        },
    }
}

pub fn load_miner_config() -> Sv2CpuMinerConfig {
    Sv2CpuMinerConfig {
        server_addr: "127.0.0.1:3333".parse().expect("Invalid server address"),
        auth_pk: Some(
            "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
                .parse()
                .expect("Invalid public key"),
        ),
        n_extended_channels: 1,
        n_standard_channels: 1,
        user_identity: "username".to_string(),
        device_id: "sv2-cpu-miner".to_string(),
        single_submit: true,
        cpu_usage_percent: 10,
        nominal_hashrate_multiplier: 1.0,
    }
}
