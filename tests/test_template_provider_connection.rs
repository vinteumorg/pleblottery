use const_sv2::{
    MESSAGE_TYPE_COINBASE_OUTPUT_CONSTRAINTS, MESSAGE_TYPE_SETUP_CONNECTION,
    MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS, MESSAGE_TYPE_SET_NEW_PREV_HASH,
};
use integration_tests_sv2::*;
use pleblottery::service::PlebLotteryService;

mod common;
use common::load_config;

#[tokio::test]
async fn test_template_provider_connection() {
    let (_tp, tp_address) = start_template_provider(None);
    let (sniffer, sniffer_addr) = start_sniffer("".to_string(), tp_address, false, None).await;

    let mut config = load_config();
    config.template_distribution_config.server_addr = sniffer_addr;

    // Give sniffer time to initialize
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone().into(),
        config.template_distribution_config.clone().into(),
    )
    .unwrap();

    pleblottery_service.start().await.unwrap();

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

    sniffer
        .wait_for_message_type(
            sniffer::MessageDirection::ToUpstream,
            MESSAGE_TYPE_COINBASE_OUTPUT_CONSTRAINTS,
        )
        .await;

    sniffer
        .wait_for_message_type(
            sniffer::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SET_NEW_PREV_HASH,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}
