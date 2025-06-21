use clap::Parser;
use tracing::{error, info};

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
    )
    .await?;

    // Use tokio::select to wait for either service completion or Ctrl+C
    tokio::select! {
        result = pleblottery_service.start() => {
            if let Err(e) = result {
                error!("Service failed to start: {}", e);
            }
        }
        result = start_web_server(&config.web_config, shared_state.clone()) => {
            if let Err(e) = result {
                error!("Web server failed to start: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
    }

    info!("Shutting down...");

    pleblottery_service.shutdown().await?;

    Ok(())
}
