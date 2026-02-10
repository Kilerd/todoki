use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::RelayRole;

/// Messages from Relay to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayToServer {
    /// Registration request
    Register {
        /// Stable relay ID (hash of machine id)
        relay_id: String,
        name: String,
        #[serde(default)]
        role: RelayRole,
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

    /// Pong response to ping
    Pong,
}

/// Messages from Server to Relay
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

    /// Permission response from server
    PermissionResponse {
        request_id: String,
        session_id: String,
        outcome: PermissionOutcome,
    },

    /// Ping for keepalive
    Ping,
}

/// Permission outcome from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionOutcome {
    /// User selected an option
    Selected { option_id: String },
    /// User cancelled
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcResult {
    Success { result: Value },
    Error { error: String },
}

impl RpcResult {
    pub fn success(value: impl Serialize) -> Self {
        RpcResult::Success {
            result: serde_json::to_value(value).unwrap_or(Value::Null),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        RpcResult::Error { error: msg.into() }
    }
}

/// Parameters for spawn-session RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionParams {
    pub agent_id: String,
    pub session_id: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Parameters for send-input RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInputParams {
    pub session_id: String,
    pub input: String,
}

/// Parameters for stop-session RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSessionParams {
    pub session_id: String,
}

/// Result for spawn-session RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionResult {
    pub pid: u32,
}
