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

    #[allow(unreachable_code)]
    Ok(())
}
