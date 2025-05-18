use pleblottery::{service::PlebLotteryService, state::SharedStateHandle};

mod common;
use common::load_config;

use std::net::{SocketAddr, TcpListener};

#[tokio::test]
async fn test_without_template_provider() {
    let mut config = load_config();

    // Dynamically bind to an available local port and immediately release it.
    //
    // This ensures the chosen port is currently unused and avoids hardcoding
    // a port, which could be occupied or behave differently across systems.
    //
    // When the PlebLotteryService tries to connect to this port, it will fail,
    // simulating an unreachable template provider in a clean and reliable way.
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to a local port");
    let unused_addr: SocketAddr = listener.local_addr().expect("Failed to get local address");
    drop(listener); // Immediately release the port so it's available (but unbound)

    config.template_distribution_config.server_addr = unused_addr;

    let shared_state: SharedStateHandle = SharedStateHandle::default();

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone().into(),
        config.template_distribution_config.clone().into(),
        shared_state,
    )
    .expect("Failed to create PlebLotteryService");

    let result = pleblottery_service.start().await;

    assert!(
        result.is_err(),
        "PlebLotteryService should fail to start when the template provider is unreachable"
    );
}
