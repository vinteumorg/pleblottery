mod cli;
mod config;
mod web;

use clap::Parser;
use tracing::info;

use config::PleblotteryConfig;
use web::server::start_web_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let args = cli::Args::parse();

    // Load configuration from file
    let config = PleblotteryConfig::from_file(args.config)?;

    info!("Config: {:?}", config);

    // todo: start the server service with mining protocol handler
    // todo: start the client service with template distribution protocol handler

    start_web_server().await?;
    info!("Web server started on http://localhost:8000");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    // todo: shutdown the server service
    // todo: shutdown the client service

    Ok(())
}
