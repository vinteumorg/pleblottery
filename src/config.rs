use serde::{Deserialize, Serialize};

use crate::lib::sv1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlebLotteryConfig {
    pub sv1: sv1::config::Sv1Config,
}

impl PlebLotteryConfig {
    pub fn new(config_path: String) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder();
        let config: config::Config = builder
            .add_source(config::File::with_name(&config_path))
            .build()?;

        let pleblottery_config: PlebLotteryConfig = config.try_deserialize()?;

        Ok(pleblottery_config)
    }
}
