use const_sv2::{MESSAGE_TYPE_SETUP_CONNECTION, MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS};
use integration_tests_sv2::*;
use pleblottery::service::PlebLotteryService;

mod common;
use common::load_config;

#[tokio::test]
async fn test_connection_with_sv2_minig_device() {
    let (_tp, tp_address) = start_template_provider(None);

    let mut config = load_config();
    config.template_distribution_config.server_addr = tp_address;

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone().into(),
        config.template_distribution_config.clone().into(),
    )
    .map_err(|e| format!("Failed to create PlebLotteryService: {}", e))
    .expect("Failed to create PlebLotteryService");

    pleblottery_service
        .start()
        .await
        .map_err(|e| format!("Failed to start PlebLotteryService: {}", e))
        .expect("Failed to start PlebLotteryService");

    let pleblottery_address = format!("0.0.0.0:{}", config.mining_server_config.listening_port);

    let (sniffer, sniffer_address) = start_sniffer(
        "sv2_device pleblottery".to_string(),
        pleblottery_address.parse().unwrap(),
        false,
        None,
    )
    .await;

    start_mining_device_sv2(sniffer_address, None, None, None, 1, None, true).await;

    sniffer
        .wait_for_message_type(
            sniffer::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SETUP_CONNECTION,
        )
        .await;
    sniffer
        .wait_for_message_type(
            sniffer::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}
