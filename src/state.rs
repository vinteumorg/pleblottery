use std::sync::Arc;

use tokio::sync::RwLock;
use tower_stratum::roles_logic_sv2::template_distribution_sv2::{NewTemplate, SetNewPrevHash};

#[derive(Default, Debug, Clone)]
/// Represents the shared state of the application, containing optional
/// information about the latest template and the latest previous hash.
///
/// # Fields
/// - `latest_template`: An optional `NewTemplate` representing the most recent template.
/// - `latest_prev_hash`: An optional `SetNewPrevHash` representing the most recent previous hash.
pub struct SharedState {
    pub latest_template: Option<NewTemplate<'static>>,
    pub latest_prev_hash: Option<SetNewPrevHash<'static>>,
}

pub type SharedStateHandle = Arc<RwLock<SharedState>>;
