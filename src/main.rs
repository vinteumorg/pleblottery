mod cli;
mod config;

use clap::Parser;
use tracing::info;

use config::PleblotteryConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let args = cli::Args::parse();

    // Load configuration from file
    let config = PleblotteryConfig::from_file(args.config)?;

    info!("Config: {:?}", config);

    // todo: start the server service with mining protocol handler
    // todo: start the client service with template distribution protocol handler

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    // todo: shutdown the server service
    // todo: shutdown the client service

    Ok(())
}
