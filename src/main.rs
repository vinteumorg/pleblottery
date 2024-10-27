#![allow(special_module_name)]

use clap::Parser;

use crate::cli::CliArgs;

mod cli;
mod config;
mod lib;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("⛏️ plebs be hashin ⚡");

    let args = CliArgs::parse();

    let pleblottery_config = config::PlebLotteryConfig::new(args.config.clone())?;

    // launch sv1 service
    let sv1_service = lib::sv1::service::Sv1Service::new(pleblottery_config.sv1).await?;
    let sv1_service_handle = sv1_service.serve();

    // container for all service handles (todo: add sv2 service)
    let service_handles = vec![sv1_service_handle];

    // join all service handles
    futures::future::join_all(service_handles).await;

    #[allow(unreachable_code)]
    Ok(())
}
