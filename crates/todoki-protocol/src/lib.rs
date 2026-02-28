//! Shared protocol definitions for todoki server and relay communication.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ============================================================================
// Relay Role
// ============================================================================

/// Relay role for task routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RelayRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl RelayRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelayRole::General => "general",
            RelayRole::Business => "business",
            RelayRole::Coding => "coding",
            RelayRole::Qa => "qa",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "business" => RelayRole::Business,
            "coding" => RelayRole::Coding,
            "qa" => RelayRole::Qa,
            _ => RelayRole::General,
        }
    }
}

// ============================================================================
// Protocol: Relay -> Server
// ============================================================================

/// Messages from Relay to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayToServer {
    /// Registration request
    Register {
        /// Stable relay ID (e.g. hash of machine id)
        relay_id: String,
        name: String,
        #[serde(default)]
        role: RelayRole,
        safe_paths: Vec<String>,
        #[serde(default)]
        labels: HashMap<String, String>,
        /// Project IDs this relay is bound to (empty = accept all)
        #[serde(default)]
        projects: Vec<Uuid>,
        /// Setup script to run before each session
        #[serde(default)]
        setup_script: Option<String>,
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

    /// Artifact detected (e.g., GitHub PR created)
    Artifact {
        session_id: String,
        agent_id: String,
        artifact_type: String,
        data: Value,
    },

    /// Prompt completed notification
    PromptCompleted {
        session_id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// Emit event to Event Bus
    /// Used by Relay to publish response events (spawn_completed, spawn_failed, etc.)
    EmitEvent { kind: String, data: Value },
}

// ============================================================================
// Protocol: Server -> Relay
// ============================================================================

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

// ============================================================================
// RPC Parameters
// ============================================================================

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
    /// Optional setup script to run before the main command.
    /// If provided, executes as: bash -c "setup_script && exec command args..."
    #[serde(default)]
    pub setup_script: Option<String>,
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

// ============================================================================
// Event Bus Events
// ============================================================================

/// Event kind constants
pub mod event_kind {
    // Permission events
    pub const PERMISSION_REQUESTED: &str = "permission.requested";
    pub const PERMISSION_RESPONDED: &str = "permission.responded";

    // Relay lifecycle events
    pub const RELAY_SPAWN_REQUESTED: &str = "relay.spawn_requested";
    pub const RELAY_SPAWN_COMPLETED: &str = "relay.spawn_completed";
    pub const RELAY_SPAWN_FAILED: &str = "relay.spawn_failed";
    pub const RELAY_INPUT_REQUESTED: &str = "relay.input_requested";
    pub const RELAY_STOP_REQUESTED: &str = "relay.stop_requested";
    pub const RELAY_PROMPT_COMPLETED: &str = "relay.prompt_completed";

    // Agent output events
    pub const RELAY_AGENT_OUTPUT: &str = "relay.agent_output";
    pub const AGENT_OUTPUT_BATCH: &str = "agent.output_batch";

    // Artifact events
    pub const ARTIFACT_CREATED: &str = "artifact.created";
}

/// Permission response outcome for event bus
/// Used in `permission.responded` event data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPermissionOutcome {
    /// User selected an option
    Selected {
        selected: String,
    },
    /// User cancelled
    Cancelled {
        cancelled: bool,
    },
}

impl EventPermissionOutcome {
    pub fn selected(option_id: impl Into<String>) -> Self {
        Self::Selected {
            selected: option_id.into(),
        }
    }

    pub fn cancelled() -> Self {
        Self::Cancelled { cancelled: true }
    }
}

/// Data for `permission.responded` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRespondedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
    pub outcome: EventPermissionOutcome,
}

/// Data for `permission.requested` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestedData {
    pub agent_id: String,
    pub session_id: String,
    pub request_id: String,
    pub tool_call_id: String,
    pub tool_call: Value,
    pub options: Value,
}

/// Data for `relay.spawn_requested` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySpawnRequestedData {
    pub relay_id: String,
    pub request_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Data for `relay.spawn_completed` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySpawnCompletedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
}

/// Data for `relay.spawn_failed` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySpawnFailedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
    pub error: String,
}

/// Data for `relay.input_requested` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayInputRequestedData {
    pub relay_id: String,
    pub session_id: String,
    pub input: String,
}

/// Data for `relay.prompt_completed` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPromptCompletedData {
    pub relay_id: String,
    pub session_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Data for `agent.output_batch` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutputBatchData {
    pub agent_id: String,
    pub session_id: String,
    pub ts: i64,
    pub stream: String,
    pub messages: Vec<String>,
}

/// Data for `artifact.created` event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCreatedData {
    pub agent_id: String,
    pub session_id: String,
    pub artifact_type: String,
    #[serde(flatten)]
    pub extra: Value,
}
