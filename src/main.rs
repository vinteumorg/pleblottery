use clap::Parser;
use tracing::info;

use pleblottery::cli;
use pleblottery::config::PleblotteryConfig;
use pleblottery::service::PlebLotteryService;
use pleblottery::state::SharedStateHandle;
use pleblottery::web::server::start_web_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let args = cli::Args::parse();

    // Load configuration from file
    let config = PleblotteryConfig::from_file(args.config)?;

    info!("Config: {:?}", config);

    let shared_state: SharedStateHandle = SharedStateHandle::default();

    let mut pleblottery_service = PlebLotteryService::new(
        config.mining_server_config,
        config.template_distribution_config,
        shared_state.clone(),
    ).await?;

    pleblottery_service.start().await?;

    start_web_server(&config.web_config, shared_state.clone()).await?;
    info!(
        "Web server started on http://localhost:{}",
        config.web_config.listening_port
    );

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");

    pleblottery_service.shutdown().await?;

    Ok(())
}
