mod manager;
mod request_tracker;

use std::collections::HashMap;

pub use manager::RelayManager;
pub use request_tracker::RequestTracker;

use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export shared protocol types from todoki-protocol
pub use todoki_protocol::AgentRole;

// ============================================================================
// Public types
// ============================================================================

/// Public info about a relay (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct RelayInfo {
    pub relay_id: String,
    pub name: String,
    pub role: String,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
    pub projects: Vec<Uuid>,
    pub setup_script: Option<String>,
    pub connected_at: i64,
    pub active_session_count: usize,
}
