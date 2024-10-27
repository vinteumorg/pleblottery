use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sv1Config {
    pub host: String,
    pub port: u16,
}
