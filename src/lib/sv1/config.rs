use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sv1Config {
    pub listen_host: String,
    pub listen_port: u16,
}
