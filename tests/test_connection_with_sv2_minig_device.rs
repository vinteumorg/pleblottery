use std::vec;

use integration_tests_sv2::*;
use pleblottery::{service::PlebLotteryService, state::SharedStateHandle};
use tower_stratum::roles_logic_sv2::{
    common_messages_sv2::{MESSAGE_TYPE_SETUP_CONNECTION, MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS},
    mining_sv2::{
        MESSAGE_TYPE_MINING_SET_NEW_PREV_HASH, MESSAGE_TYPE_NEW_MINING_JOB,
        MESSAGE_TYPE_OPEN_MINING_CHANNEL_ERROR, MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL,
        MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL_SUCCESS,
    },
    template_distribution_sv2::{MESSAGE_TYPE_NEW_TEMPLATE, MESSAGE_TYPE_SET_NEW_PREV_HASH},
};

mod common;
use common::load_config;

#[tokio::test]
async fn test_connection_with_sv2_minig_device() {
    start_tracing();
    let (_tp, tp_address) = start_template_provider(None);

    let mut config = load_config();
    config.template_distribution_config.server_addr = tp_address;

    let shared_state: SharedStateHandle = SharedStateHandle::default();

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone(),
        config.template_distribution_config.clone(),
        shared_state,
    )
    .await
    .expect("Failed to create PlebLotteryService");

    let mut pleblottery_service_clone = pleblottery_service.clone();
    tokio::spawn(async move {
        pleblottery_service_clone.start().await.unwrap();
    });

    // wait for the service to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let pleblottery_address = format!("0.0.0.0:{}", config.mining_server_config.listening_port);

    let (sniffer, sniffer_address) = start_sniffer(
        "sv2_device pleblottery",
        pleblottery_address.parse().unwrap(),
        false,
        vec![],
    );

    start_mining_device_sv2(sniffer_address, None, None, None, 1, None, true);

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
            MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL_SUCCESS,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_NEW_MINING_JOB,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_MINING_SET_NEW_PREV_HASH,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_with_sv2_minig_device_when_tp_not_send_new_template() {
    start_tracing();
    let (_tp, tp_address) = start_template_provider(None);

    let ignore_new_template = interceptor::IgnoreMessage::new(
        interceptor::MessageDirection::ToDownstream,
        MESSAGE_TYPE_NEW_TEMPLATE,
    );

    let (tp_sniffer, tp_sniffer_addr) = start_sniffer(
        "tp pleblottery",
        tp_address,
        false,
        vec![ignore_new_template.into()],
    );

    let mut config = load_config();
    config.template_distribution_config.server_addr = tp_sniffer_addr;

    // Set a high expected shares per minute to ensure we can submit shares quickly
    config.mining_server_config.expected_shares_per_minute = 100.0;

    // Give sniffer time to initialize
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let shared_state: SharedStateHandle = pleblottery::state::SharedStateHandle::default();

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

    let pleblottery_address = format!("0.0.0.0:{}", config.mining_server_config.listening_port);

    let (sniffer, sniffer_address) = start_sniffer(
        "sv2_device pleblottery",
        pleblottery_address.parse().unwrap(),
        false,
        vec![],
    );

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SET_NEW_PREV_HASH,
        )
        .await;

    start_mining_device_sv2(sniffer_address, None, None, None, 1, None, true);

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_OPEN_MINING_CHANNEL_ERROR,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}

// In this case here, the template provider will send the NewTemplate message, but never activate it
// because we are making the sniffer ignore the NewPrevHash message.
#[tokio::test]
async fn test_connection_with_sv2_minig_device_when_tp_not_send_new_prev_hash() {
    start_tracing();
    let (_tp, tp_address) = start_template_provider(None);

    let ignore_new_prev_hash = interceptor::IgnoreMessage::new(
        interceptor::MessageDirection::ToDownstream,
        MESSAGE_TYPE_SET_NEW_PREV_HASH,
    );

    let (tp_sniffer, tp_sniffer_addr) = start_sniffer(
        "tp pleblottery",
        tp_address,
        false,
        vec![ignore_new_prev_hash.into()],
    );

    let mut config = load_config();
    config.template_distribution_config.server_addr = tp_sniffer_addr;

    // Set a high expected shares per minute to ensure we can submit shares quickly
    config.mining_server_config.expected_shares_per_minute = 100.0;

    // Give sniffer time to initialize
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let shared_state: SharedStateHandle = pleblottery::state::SharedStateHandle::default();

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

    let pleblottery_address = format!("0.0.0.0:{}", config.mining_server_config.listening_port);

    let (sniffer, sniffer_address) = start_sniffer(
        "sv2_device pleblottery",
        pleblottery_address.parse().unwrap(),
        false,
        vec![],
    );

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_NEW_TEMPLATE,
        )
        .await;

    start_mining_device_sv2(sniffer_address, None, None, None, 1, None, true);

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_OPEN_MINING_CHANNEL_ERROR,
        )
        .await;

    pleblottery_service.shutdown().await.unwrap();
}
