use binary_codec_sv2::{Decodable, Encodable};
use integration_tests_sv2::{interceptor, start_sniffer, start_template_provider};
use pleblottery::web::server::start_web_server;
use pleblottery::{service::PlebLotteryService, state::SharedStateHandle};
use reqwest::Client;
use sv2_services::roles_logic_sv2::template_distribution_sv2::MESSAGE_TYPE_NEW_TEMPLATE;
mod common;
use common::load_config;
use sv2_services::roles_logic_sv2::parsers::IsSv2Message;
use sv2_services::roles_logic_sv2::template_distribution_sv2::SetNewPrevHash;

/// Integration test to verify that the shared state between the PlebLotteryService
/// and the web server works as expected.
///
/// The test sets up:
/// 1. A simulated Template Provider (with a sniffer to inspect messages),
/// 2. A PlebLotteryService using shared state,
/// 3. A web server exposing an API backed by the same shared state.
///
/// It checks that after the mining service receives a `SetNewPrevHash` message,
/// the web server exposes the same data through its `/api/latest-prev-hash` endpoint.
///
/// This validates that both components access and reflect the same internal state.
#[tokio::test]
async fn test_shared_state_between_service_and_web() {
    // Start a simulated Template Provider and attach a sniffer to monitor its messages
    let (_tp, tp_address) = start_template_provider(None);
    let (tp_sniffer, tp_sniffer_addr) = start_sniffer("", tp_address, false, vec![]);
    let mut config = load_config();
    config.template_distribution_config.server_addr = tp_sniffer_addr;

    // Give sniffer some time to initialize before starting the service
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Initialize and start the mining service with the shared state
    let shared_state: SharedStateHandle = SharedStateHandle::default();

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone(),
        config.template_distribution_config.clone(),
        shared_state.clone(),
    )
    .await
    .expect("Failed to create PlebLotteryService");

    let mut pleblottery_service_clone = pleblottery_service.clone();
    tokio::spawn(async move {
        pleblottery_service_clone.start().await.unwrap();
    });

    // wait for the service to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Start the web server with the same shared state
    let web_config = config.web_config.clone();
    tokio::spawn(async move {
        start_web_server(&web_config, shared_state.clone())
            .await
            .unwrap();
    });

    // Wait until a `NewTemplate` message is seen going downstream (from TP to service)
    tp_sniffer
        .wait_for_message_type_and_clean_queue(
            interceptor::MessageDirection::ToDownstream,
            MESSAGE_TYPE_NEW_TEMPLATE,
        )
        .await;

    // Extract the upstream message and parse it as SetNewPrevHash
    let message = tp_sniffer.next_message_from_upstream().unwrap().1;
    let mut dst = vec![0; message.message_type() as usize];
    let _ = message.clone().to_bytes(&mut dst);
    let set_new_prev_hash = SetNewPrevHash::from_bytes(&mut dst).unwrap();

    // Query the web server for the latest prev_hash via the API
    let client = Client::new();
    let resp = client
        .get(format!(
            "http://localhost:{}/api/latest-prev-hash",
            config.web_config.listening_port
        ))
        .send()
        .await
        .expect("Failed to query web server");

    // Ensure the server responds successfully
    assert!(
        resp.status().is_success(),
        "Web server returned non-200 status."
    );

    let resp_text = resp.text().await.expect("Failed to read response text");

    // Assert the response from the web server contains the expected values from the SetNewPrevHash message that was shared via SharedState
    // This tell us that the web server is correctly reflecting the state updated by the SV2 Handlers

    assert!(
        resp_text.contains(
            &set_new_prev_hash
                .prev_hash
                .to_vec()
                .iter()
                .rev()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
                .to_string()
        ),
        "Response does not contain expected prev_hash."
    );

    assert!(
        resp_text.contains(&format!("{}", set_new_prev_hash.template_id)),
        "Response does not contain expected template_id."
    );

    assert!(
        resp_text.contains(&format!("{:02x}", set_new_prev_hash.n_bits).to_string()),
        "Response does not contain expected n_bits."
    );

    assert!(
        resp_text.contains(
            &set_new_prev_hash
                .target
                .to_vec()
                .iter()
                .rev()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
                .to_string()
        ),
        "Response does not contain expected target."
    );

    // Gracefully shut down the mining service
    pleblottery_service.shutdown().await.unwrap();
}
