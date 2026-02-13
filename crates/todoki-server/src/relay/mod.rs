mod broadcaster;
mod manager;

use std::collections::HashMap;

pub use broadcaster::{AgentBroadcaster, AgentStreamEvent};
pub use manager::RelayManager;

use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// Re-export shared protocol types from todoki-protocol
pub use todoki_protocol::{PermissionOutcome, RelayRole, RelayToServer, RpcResult, ServerToRelay};

#[derive(Debug)]
pub struct RpcResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl From<RpcResult> for RpcResponse {
    fn from(result: RpcResult) -> Self {
        match result {
            RpcResult::Success { result } => RpcResponse {
                success: true,
                result: Some(result),
                error: None,
            },
            RpcResult::Error { error } => RpcResponse {
                success: false,
                result: None,
                error: Some(error),
            },
        }
    }
}

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

/// Agent output event (for broadcasting to clients)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutputEvent {
    pub agent_id: String,
    pub session_id: String,
    pub seq: i64,
    pub ts: i64,
    pub stream: String,
    pub message: String,
}

/// Session status event (for broadcasting to clients)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusEvent {
    pub session_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
}
