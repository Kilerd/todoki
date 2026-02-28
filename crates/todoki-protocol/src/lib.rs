//! Shared protocol definitions for todoki server and relay communication.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Agent Role
// ============================================================================

/// Agent role for task routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::General => "general",
            AgentRole::Business => "business",
            AgentRole::Coding => "coding",
            AgentRole::Qa => "qa",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "business" => AgentRole::Business,
            "coding" => AgentRole::Coding,
            "qa" => AgentRole::Qa,
            _ => AgentRole::General,
        }
    }
}

// ============================================================================
// Relay Session Parameters (used by relay internally)
// ============================================================================

/// Parameters for spawn-session
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

/// Parameters for send-input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInputParams {
    pub session_id: String,
    pub input: String,
}

/// Result for spawn-session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionResult {
    pub pid: u32,
}

// ============================================================================
// Event Bus Events
// ============================================================================

/// Event kind constants
///
/// Event kinds follow the format: `<category>.<action>`
///
/// Categories:
/// - task: Task lifecycle events
/// - agent: Agent lifecycle and collaboration events
/// - artifact: External artifacts (PRs, issues, etc.)
/// - permission: Permission request/response events
/// - relay: Relay communication events
/// - system: System-level events
pub mod event_kind {
    // ========================================================================
    // Task lifecycle
    // ========================================================================
    pub const TASK_CREATED: &str = "task.created";
    pub const TASK_STATUS_CHANGED: &str = "task.status_changed";
    pub const TASK_ASSIGNED: &str = "task.assigned";
    pub const TASK_COMPLETED: &str = "task.completed";
    pub const TASK_FAILED: &str = "task.failed";
    pub const TASK_ARCHIVED: &str = "task.archived";

    // ========================================================================
    // Agent lifecycle
    // ========================================================================
    pub const AGENT_REGISTERED: &str = "agent.registered";
    pub const AGENT_STARTED: &str = "agent.started";
    pub const AGENT_STOPPED: &str = "agent.stopped";
    pub const AGENT_OUTPUT: &str = "agent.output";
    pub const AGENT_OUTPUT_BATCH: &str = "agent.output_batch";
    pub const AGENT_ERROR: &str = "agent.error";

    // ========================================================================
    // Agent collaboration (PM → BA → Coding → QA pipeline)
    // ========================================================================
    pub const REQUIREMENT_ANALYZED: &str = "agent.requirement_analyzed";
    pub const BUSINESS_CONTEXT_READY: &str = "agent.business_context_ready";
    pub const CODE_REVIEW_REQUESTED: &str = "agent.code_review_requested";
    pub const QA_TEST_PASSED: &str = "agent.qa_test_passed";
    pub const QA_TEST_FAILED: &str = "agent.qa_test_failed";

    // ========================================================================
    // Agent Session Events
    // ========================================================================
    pub const AGENT_SESSION_STARTED: &str = "agent.session_started";
    pub const AGENT_SESSION_EXITED: &str = "agent.session_exited";

    // ========================================================================
    // Artifacts
    // ========================================================================
    pub const ARTIFACT_CREATED: &str = "artifact.created";
    pub const GITHUB_PR_OPENED: &str = "artifact.github_pr_opened";
    pub const GITHUB_PR_MERGED: &str = "artifact.github_pr_merged";

    // ========================================================================
    // Permission
    // ========================================================================
    pub const PERMISSION_REQUESTED: &str = "permission.requested";
    pub const PERMISSION_RESPONDED: &str = "permission.responded";
    pub const PERMISSION_APPROVED: &str = "permission.approved";
    pub const PERMISSION_DENIED: &str = "permission.denied";

    // ========================================================================
    // Relay Lifecycle
    // ========================================================================
    pub const RELAY_UP: &str = "relay.up";
    pub const RELAY_DOWN: &str = "relay.down";

    // ========================================================================
    // Relay Data Upload (Relay → Server)
    // ========================================================================
    pub const RELAY_AGENT_OUTPUT: &str = "relay.agent_output";
    pub const RELAY_AGENT_OUTPUT_BATCH: &str = "relay.agent_output_batch";
    pub const RELAY_SESSION_STATUS: &str = "relay.session_status";
    pub const RELAY_PERMISSION_REQUEST: &str = "relay.permission_request";
    pub const RELAY_ARTIFACT: &str = "relay.artifact";
    pub const RELAY_PROMPT_COMPLETED: &str = "relay.prompt_completed";

    // ========================================================================
    // Relay Commands (Server → Relay)
    // ========================================================================
    pub const RELAY_SPAWN_REQUESTED: &str = "relay.spawn_requested";
    pub const RELAY_STOP_REQUESTED: &str = "relay.stop_requested";
    pub const RELAY_INPUT_REQUESTED: &str = "relay.input_requested";

    // ========================================================================
    // Relay Responses (Relay → Server)
    // ========================================================================
    pub const RELAY_SPAWN_COMPLETED: &str = "relay.spawn_completed";
    pub const RELAY_SPAWN_FAILED: &str = "relay.spawn_failed";
    pub const RELAY_STOP_COMPLETED: &str = "relay.stop_completed";

    // ========================================================================
    // System
    // ========================================================================
    pub const RELAY_CONNECTED: &str = "system.relay_connected";
    pub const RELAY_DISCONNECTED: &str = "system.relay_disconnected";
}

/// Permission response outcome for event bus
/// Used in `permission.responded` event data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPermissionOutcome {
    /// User selected an option
    Selected { selected: String },
    /// User cancelled
    Cancelled { cancelled: bool },
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
