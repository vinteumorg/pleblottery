#[derive(clap::Parser)]
#[command(about = "a hashrate aggregator for a pleb-friendly and fully sovereign solo/lottery Bitcoin mining experience", author = env!("CARGO_PKG_AUTHORS"), version = env!("CARGO_PKG_VERSION"))]
pub struct CliArgs {
    #[arg(
        short,
        long,
        help = "path for config file",
        default_value = "config.toml"
    )]
    pub config: String,
}
