mod broadcaster;
mod manager;

use std::collections::HashMap;

pub use broadcaster::{AgentBroadcaster, AgentStreamEvent};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use manager::RelayManager;

// ============================================================================
// Protocol: Relay -> Server
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayToServer {
    /// Registration request
    Register {
        name: String,
        safe_paths: Vec<String>,
        #[serde(default)]
        labels: HashMap<String, String>,
    },

    /// RPC response
    RpcResponse {
        id: String,
        #[serde(flatten)]
        result: RpcResult,
    },

    /// Agent output forwarding
    AgentOutput {
        agent_id: String,
        session_id: String,
        seq: i64,
        ts: i64,
        stream: String,
        message: String,
    },

    /// Session status change
    SessionStatus {
        session_id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },

    /// Permission request from ACP agent
    PermissionRequest {
        request_id: String,
        agent_id: String,
        session_id: String,
        tool_call_id: String,
        options: Value,
        tool_call: Value,
    },

    /// Pong response
    Pong,
}

// ============================================================================
// Protocol: Server -> Relay
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToRelay {
    /// Registration confirmed
    Registered { relay_id: String },

    /// RPC request
    RpcRequest {
        id: String,
        method: String,
        params: Value,
    },

    /// Permission response to relay
    PermissionResponse {
        request_id: String,
        session_id: String,
        outcome: PermissionOutcome,
    },

    /// Ping for keepalive
    Ping,
}

/// Permission outcome from user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionOutcome {
    /// User selected an option
    Selected { option_id: String },
    /// User cancelled
    Cancelled,
}

// ============================================================================
// RPC types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcResult {
    Success { result: Value },
    Error { error: String },
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayInfo {
    pub relay_id: String,
    pub name: String,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
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
