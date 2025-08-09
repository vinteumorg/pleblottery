use std::vec;

use integration_tests_sv2::*;
use pleblottery::{service::PlebLotteryService, state::SharedStateHandle};
use sv2_services::roles_logic_sv2::mining_sv2::{
    MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED, MESSAGE_TYPE_SUBMIT_SHARES_STANDARD,
    MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS,
};
use sv2_services::roles_logic_sv2::template_distribution_sv2::{
    MESSAGE_TYPE_NEW_TEMPLATE, MESSAGE_TYPE_SET_NEW_PREV_HASH, MESSAGE_TYPE_SUBMIT_SOLUTION,
};

mod common;
use common::{load_config, load_miner_config};

#[tokio::test]
async fn test_submit_share_and_submit_solution() {
    start_tracing();
    let (_tp, tp_address) = start_template_provider(None);
    let (tp_sniffer, tp_sniffer_addr) = start_sniffer("tp pleblottery", tp_address, false, vec![]);

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

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SET_NEW_PREV_HASH,
        )
        .await;

    let mut miner_config = load_miner_config();
    miner_config.server_addr = sniffer_address;
    miner_config.n_extended_channels = 0;
    tokio::spawn(async move {
        sv2_cpu_miner::client::Sv2CpuMiner::new(miner_config)
            .await
            .unwrap()
            .start()
            .await
            .unwrap();
    });

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SUBMIT_SHARES_STANDARD,
        )
        .await;

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SUBMIT_SOLUTION,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS,
        )
        .await;
    pleblottery_service.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_submit_share_and_submit_solution_for_extended_channels() {
    start_tracing();
    let (_tp, tp_address) = start_template_provider(None);
    let (tp_sniffer, tp_sniffer_addr) = start_sniffer("tp pleblottery", tp_address, false, vec![]);

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

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SET_NEW_PREV_HASH,
        )
        .await;

    let mut miner_config = load_miner_config();
    miner_config.server_addr = sniffer_address;
    miner_config.n_standard_channels = 0;
    tokio::spawn(async move {
        sv2_cpu_miner::client::Sv2CpuMiner::new(miner_config)
            .await
            .unwrap()
            .start()
            .await
            .unwrap();
    });

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED,
        )
        .await;

    tp_sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToUpstream,
            MESSAGE_TYPE_SUBMIT_SOLUTION,
        )
        .await;

    sniffer
        .wait_for_message_type(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS,
        )
        .await;
    pleblottery_service.shutdown().await.unwrap();
}
