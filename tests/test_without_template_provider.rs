use pleblottery::service::PlebLotteryService;

mod common;
use common::load_config;

#[tokio::test]
async fn test_without_template_provider() {
    let config = load_config();

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config.clone().into(),
        config.template_distribution_config.clone().into(),
    )
    .expect("Failed to create PlebLotteryService");

    let result = pleblottery_service.start().await;
    assert!(
        result.is_err(),
        "PlebLotteryService should not start without a template provider"
    );
}
