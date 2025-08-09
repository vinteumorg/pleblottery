use integration_tests_sv2::*;
use pleblottery::{service::PlebLotteryService, state::SharedStateHandle};
use sv2_services::roles_logic_sv2::common_messages_sv2::{
    MESSAGE_TYPE_SETUP_CONNECTION, MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
};
use sv2_services::roles_logic_sv2::template_distribution_sv2::{
    MESSAGE_TYPE_COINBASE_OUTPUT_CONSTRAINTS, MESSAGE_TYPE_SET_NEW_PREV_HASH,
};

mod common;
use common::load_config;

#[tokio::test]
async fn test_template_provider_connection() {
    let (_tp, tp_address) = start_template_provider(None);
    let (sniffer, sniffer_addr) = start_sniffer("", tp_address, false, vec![]);

    let mut config = load_config();
    config.template_distribution_config.server_addr = sniffer_addr;

    let shared_state: SharedStateHandle = pleblottery::state::SharedStateHandle::default();

    // Give sniffer time to initialize
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone(),
        config.template_distribution_config.clone(),
        shared_state,
    )
    .await
    .unwrap();

    let mut pleblottery_service_clone = pleblottery_service.clone();
    tokio::spawn(async move {
        pleblottery_service_clone.start().await.unwrap();
    });
    // wait for the service to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SETUP_CONNECTION,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_COINBASE_OUTPUT_CONSTRAINTS,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SET_NEW_PREV_HASH,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}
