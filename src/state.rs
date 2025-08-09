use std::{collections::HashMap, sync::Arc};

use sv2_services::roles_logic_sv2::template_distribution_sv2::{NewTemplate, SetNewPrevHash};
use tokio::sync::RwLock;

use crate::sv2_handlers::mining_server_handler::PleblotteryMiningClient;

#[derive(Default, Debug, Clone)]
/// Represents the state of the application (shared with the web server), containing optional
/// information about the latest template and the latest previous hash.
pub struct SharedState {
    pub latest_template: Option<NewTemplate<'static>>,
    pub latest_prev_hash: Option<SetNewPrevHash<'static>>,
    pub total_clients: u32,
    pub total_shares_submitted: u64,
    pub best_share: f64,
    pub total_hashrate: f32,
    pub blocks_found: u64,
    pub clients: Arc<RwLock<HashMap<u32, Arc<RwLock<PleblotteryMiningClient>>>>>,
}
impl SharedState {
    pub fn format_best_share(&self) -> String {
        let (value, suffix) = if self.best_share >= 1_000_000_000.0 {
            (self.best_share / 1_000_000_000.0, "B")
        } else if self.best_share >= 1_000_000.0 {
            (self.best_share / 1_000_000.0, "M")
        } else if self.best_share >= 1_000.0 {
            (self.best_share / 1_000.0, "K")
        } else {
            (self.best_share, "")
        };
        format!("{:.2}{}", value, suffix)
    }
    pub fn format_hashrate(&self) -> String {
        let (value, unit) = if self.total_hashrate >= 1e12 {
            (self.total_hashrate / 1e12, "Th/s")
        } else if self.total_hashrate >= 1e9 {
            (self.total_hashrate / 1e9, "Gh/s")
        } else if self.total_hashrate >= 1e6 {
            (self.total_hashrate / 1e6, "Mh/s")
        } else if self.total_hashrate >= 1e3 {
            (self.total_hashrate / 1e3, "Kh/s")
        } else {
            (self.total_hashrate, "h/s")
        };
        format!("{:.2} {}", value, unit)
    }
}

pub type SharedStateHandle = Arc<RwLock<SharedState>>;
