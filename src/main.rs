#![allow(special_module_name)]

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use crate::cli::CliArgs;

mod cli;
mod config;
mod lib;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Configure tracing
    let subscriber = Registry::default();

    #[cfg(feature = "tokio_debug")]
    {
        // Layer for tokio-console
        let console_layer = console_subscriber::spawn();

        // Layer for standard tracing output with a filter
        let fmt_layer = fmt::Layer::default()
            .with_filter(EnvFilter::new("debug")); // Only show DEBUG and above

        // Combine both layers
        let combined = subscriber.with(console_layer).with(fmt_layer);
        tracing::subscriber::set_global_default(combined)
            .expect("Failed to set subscriber");
    }

    #[cfg(not(feature = "tokio_debug"))]
    {
        // Layer for standard tracing output with a filter
        let fmt_layer = fmt::Layer::default()
            .with_filter(EnvFilter::new("info")); // Only show INFO and above
        let combined = subscriber.with(fmt_layer);
        tracing::subscriber::set_global_default(combined)
            .expect("Failed to set subscriber");
    }

    // Log a test message
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
