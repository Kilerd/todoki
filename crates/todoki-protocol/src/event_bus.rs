//! Structured event bus message definitions.
//!
//! All event types and their data structures are defined here for use by both
//! todoki-server and todoki-relay.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Event Kind Constants
// ============================================================================

/// Event kind constants following the format: `<category>.<action>`
pub struct EventKind;

impl EventKind {
    // Task lifecycle
    pub const TASK_CREATED: &str = "task.created";
    pub const TASK_STATUS_CHANGED: &str = "task.status_changed";
    pub const TASK_ASSIGNED: &str = "task.assigned";
    pub const TASK_COMPLETED: &str = "task.completed";
    pub const TASK_FAILED: &str = "task.failed";
    pub const TASK_ARCHIVED: &str = "task.archived";

    // Agent lifecycle
    pub const AGENT_REGISTERED: &str = "agent.registered";
    pub const AGENT_STARTED: &str = "agent.started";
    pub const AGENT_STOPPED: &str = "agent.stopped";
    pub const AGENT_OUTPUT: &str = "agent.output";
    pub const AGENT_OUTPUT_BATCH: &str = "agent.output_batch";
    pub const AGENT_ERROR: &str = "agent.error";

    // Agent collaboration
    pub const REQUIREMENT_ANALYZED: &str = "agent.requirement_analyzed";
    pub const BUSINESS_CONTEXT_READY: &str = "agent.business_context_ready";
    pub const CODE_REVIEW_REQUESTED: &str = "agent.code_review_requested";
    pub const QA_TEST_PASSED: &str = "agent.qa_test_passed";
    pub const QA_TEST_FAILED: &str = "agent.qa_test_failed";

    // Agent session
    pub const AGENT_SESSION_STARTED: &str = "agent.session_started";
    pub const AGENT_SESSION_EXITED: &str = "agent.session_exited";

    // Artifacts
    pub const ARTIFACT_CREATED: &str = "artifact.created";
    pub const GITHUB_PR_OPENED: &str = "artifact.github_pr_opened";
    pub const GITHUB_PR_MERGED: &str = "artifact.github_pr_merged";

    // Permission
    pub const PERMISSION_REQUESTED: &str = "permission.requested";
    pub const PERMISSION_RESPONDED: &str = "permission.responded";
    pub const PERMISSION_APPROVED: &str = "permission.approved";
    pub const PERMISSION_DENIED: &str = "permission.denied";
    pub const PERMISSION_REVOKED: &str = "permission.revoked";
    pub const PERMISSION_EXPIRED: &str = "permission.expired";
    pub const PERMISSION_CANCELLED: &str = "permission.cancelled";

    // Relay lifecycle
    pub const RELAY_UP: &str = "relay.up";
    pub const RELAY_DOWN: &str = "relay.down";

    // Relay data upload (Relay → Server)
    pub const RELAY_AGENT_OUTPUT: &str = "relay.agent_output";
    pub const RELAY_AGENT_OUTPUT_BATCH: &str = "relay.agent_output_batch";
    pub const RELAY_SESSION_STATUS: &str = "relay.session_status";
    pub const RELAY_PERMISSION_REQUEST: &str = "relay.permission_request";
    pub const RELAY_ARTIFACT: &str = "relay.artifact";
    pub const RELAY_PROMPT_COMPLETED: &str = "relay.prompt_completed";
    pub const RELAY_ERROR: &str = "relay.error";

    // Relay commands (Server → Relay)
    pub const RELAY_SPAWN_REQUESTED: &str = "relay.spawn_requested";
    pub const RELAY_STOP_REQUESTED: &str = "relay.stop_requested";
    pub const RELAY_INPUT_REQUESTED: &str = "relay.input_requested";

    // Relay responses (Relay → Server)
    pub const RELAY_SPAWN_COMPLETED: &str = "relay.spawn_completed";
    pub const RELAY_SPAWN_FAILED: &str = "relay.spawn_failed";
    pub const RELAY_STOP_COMPLETED: &str = "relay.stop_completed";

    // System
    pub const SYSTEM_RELAY_CONNECTED: &str = "system.relay_connected";
    pub const SYSTEM_RELAY_DISCONNECTED: &str = "system.relay_disconnected";
}

// ============================================================================
// Permission Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionOption {
    pub kind: String,
    pub name: String,
    pub option_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub title: String,
    pub raw_input: Value,
    pub tool_call_id: Option<String>,
}

/// Selected outcome payload with option_id
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectedOutcome {
    pub option_id: String,
}

/// Permission response outcome
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionOutcome {
    Selected { selected: SelectedOutcome },
    Cancelled { cancelled: bool },
}

impl PermissionOutcome {
    pub fn selected(option_id: impl Into<String>) -> Self {
        Self::Selected {
            selected: SelectedOutcome {
                option_id: option_id.into(),
            },
        }
    }

    pub fn cancelled() -> Self {
        Self::Cancelled { cancelled: true }
    }
}

// ============================================================================
// Task Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCreatedData {
    pub task_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStatusChangedData {
    pub task_id: String,
    pub old_status: String,
    pub new_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAssignedData {
    pub task_id: String,
    pub assigned_agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCompletedData {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskFailedData {
    pub task_id: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskArchivedData {
    pub task_id: String,
}

// ============================================================================
// Agent Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRegisteredData {
    pub agent_id: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStartedData {
    pub agent_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStoppedData {
    pub agent_id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutputData {
    pub agent_id: String,
    pub session_id: String,
    pub stream: String,
    pub message: String,
    pub ts: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutputBatchData {
    pub session_id: String,
    pub stream: String,
    pub messages: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentErrorData {
    pub agent_id: String,
    pub session_id: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionStartedData {
    pub agent_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionExitedData {
    pub agent_id: String,
    pub session_id: String,
    pub exit_code: Option<i32>,
}

// ============================================================================
// Agent Collaboration Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequirementAnalyzedData {
    pub agent_id: String,
    pub task_id: String,
    pub analysis: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusinessContextReadyData {
    pub agent_id: String,
    pub task_id: String,
    pub context: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeReviewRequestedData {
    pub agent_id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QaTestResultData {
    pub agent_id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

// ============================================================================
// Artifact Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactCreatedData {
    pub session_id: String,
    pub artifact_type: String,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GithubPrData {
    pub task_id: String,
    pub pr_url: String,
    pub pr_number: i64,
    pub repo: String,
}

// ============================================================================
// Relay Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayLifecycleData {
    pub relay_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayAgentOutputData {
    pub relay_id: String,
    pub target_agent_id: String,
    pub session_id: String,
    pub stream: String,
    pub message: String,
    pub ts: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayAgentOutputBatchData {
    pub relay_id: String,
    pub target_agent_id: String,
    pub session_id: String,
    pub stream: String,
    pub messages: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySessionStatusData {
    pub relay_id: String,
    pub session_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayPermissionRequestData {
    pub relay_id: String,
    pub target_agent_id: String,
    pub session_id: String,
    pub request_id: String,
    pub tool_call_id: String,
    pub tool_call: Value,
    pub options: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayArtifactData {
    pub relay_id: String,
    pub target_agent_id: String,
    pub session_id: String,
    pub artifact_type: String,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayPromptCompletedData {
    pub relay_id: String,
    pub session_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayErrorData {
    pub relay_id: String,
    pub session_id: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySpawnRequestedData {
    pub relay_id: String,
    pub request_id: String,
    pub target_agent_id: String,
    pub session_id: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayStopRequestedData {
    pub relay_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayInputRequestedData {
    pub relay_id: String,
    pub session_id: String,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySpawnCompletedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySpawnFailedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayStopCompletedData {
    pub relay_id: String,
    pub session_id: String,
}

// ============================================================================
// System Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemRelayConnectionData {
    pub relay_id: String,
}

// ============================================================================
// Permission Event Data (for permission.requested/responded)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionRequestedData {
    pub session_id: String,
    pub request_id: String,
    pub tool_call_id: String,
    pub tool_call: Value,
    pub options: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionRespondedData {
    pub relay_id: String,
    pub request_id: String,
    pub session_id: String,
    pub outcome: PermissionOutcome,
}

// ============================================================================
// Builtin Event Enum
// ============================================================================

/// All builtin event types with their structured data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum BuiltinEvent {
    // Task events
    #[serde(rename = "task.created")]
    TaskCreated(TaskCreatedData),
    #[serde(rename = "task.status_changed")]
    TaskStatusChanged(TaskStatusChangedData),
    #[serde(rename = "task.assigned")]
    TaskAssigned(TaskAssignedData),
    #[serde(rename = "task.completed")]
    TaskCompleted(TaskCompletedData),
    #[serde(rename = "task.failed")]
    TaskFailed(TaskFailedData),
    #[serde(rename = "task.archived")]
    TaskArchived(TaskArchivedData),

    // Agent lifecycle events
    #[serde(rename = "agent.registered")]
    AgentRegistered(AgentRegisteredData),
    #[serde(rename = "agent.started")]
    AgentStarted(AgentStartedData),
    #[serde(rename = "agent.stopped")]
    AgentStopped(AgentStoppedData),
    #[serde(rename = "agent.output")]
    AgentOutput(AgentOutputData),
    #[serde(rename = "agent.output_batch")]
    AgentOutputBatch(AgentOutputBatchData),
    #[serde(rename = "agent.error")]
    AgentError(AgentErrorData),
    #[serde(rename = "agent.session_started")]
    AgentSessionStarted(AgentSessionStartedData),
    #[serde(rename = "agent.session_exited")]
    AgentSessionExited(AgentSessionExitedData),

    // Agent collaboration events
    #[serde(rename = "agent.requirement_analyzed")]
    RequirementAnalyzed(RequirementAnalyzedData),
    #[serde(rename = "agent.business_context_ready")]
    BusinessContextReady(BusinessContextReadyData),
    #[serde(rename = "agent.code_review_requested")]
    CodeReviewRequested(CodeReviewRequestedData),
    #[serde(rename = "agent.qa_test_passed")]
    QaTestPassed(QaTestResultData),
    #[serde(rename = "agent.qa_test_failed")]
    QaTestFailed(QaTestResultData),

    // Artifact events
    #[serde(rename = "artifact.created")]
    ArtifactCreated(ArtifactCreatedData),
    #[serde(rename = "artifact.github_pr_opened")]
    GithubPrOpened(GithubPrData),
    #[serde(rename = "artifact.github_pr_merged")]
    GithubPrMerged(GithubPrData),

    // Permission events
    #[serde(rename = "permission.requested")]
    PermissionRequested(PermissionRequestedData),
    #[serde(rename = "permission.responded")]
    PermissionResponded(PermissionRespondedData),
    #[serde(rename = "permission.approved")]
    PermissionApproved { request_id: String },
    #[serde(rename = "permission.denied")]
    PermissionDenied { request_id: String },
    #[serde(rename = "permission.revoked")]
    PermissionRevoked { request_id: String },
    #[serde(rename = "permission.expired")]
    PermissionExpired { request_id: String },
    #[serde(rename = "permission.cancelled")]
    PermissionCancelled { request_id: String },

    // Relay lifecycle events
    #[serde(rename = "relay.up")]
    RelayUp(RelayLifecycleData),
    #[serde(rename = "relay.down")]
    RelayDown(RelayLifecycleData),

    // Relay data events (Relay → Server)
    #[serde(rename = "relay.agent_output")]
    RelayAgentOutput(RelayAgentOutputData),
    #[serde(rename = "relay.agent_output_batch")]
    RelayAgentOutputBatch(RelayAgentOutputBatchData),
    #[serde(rename = "relay.session_status")]
    RelaySessionStatus(RelaySessionStatusData),
    #[serde(rename = "relay.permission_request")]
    RelayPermissionRequest(RelayPermissionRequestData),
    #[serde(rename = "relay.artifact")]
    RelayArtifact(RelayArtifactData),
    #[serde(rename = "relay.prompt_completed")]
    RelayPromptCompleted(RelayPromptCompletedData),
    #[serde(rename = "relay.error")]
    RelayError(RelayErrorData),

    // Relay command events (Server → Relay)
    #[serde(rename = "relay.spawn_requested")]
    RelaySpawnRequested(RelaySpawnRequestedData),
    #[serde(rename = "relay.stop_requested")]
    RelayStopRequested(RelayStopRequestedData),
    #[serde(rename = "relay.input_requested")]
    RelayInputRequested(RelayInputRequestedData),

    // Relay response events (Relay → Server)
    #[serde(rename = "relay.spawn_completed")]
    RelaySpawnCompleted(RelaySpawnCompletedData),
    #[serde(rename = "relay.spawn_failed")]
    RelaySpawnFailed(RelaySpawnFailedData),
    #[serde(rename = "relay.stop_completed")]
    RelayStopCompleted(RelayStopCompletedData),

    // System events
    #[serde(rename = "system.relay_connected")]
    SystemRelayConnected(SystemRelayConnectionData),
    #[serde(rename = "system.relay_disconnected")]
    SystemRelayDisconnected(SystemRelayConnectionData),
}

// ============================================================================
// Event Wrapper Types
// ============================================================================

/// Event that can be either builtin or custom
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Event {
    Builtin(BuiltinEvent),
    Custom { kind: String, data: Value },
}

/// Complete event bus message with agent_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    #[serde(flatten)]
    pub event: Event,
    /// which agent emitted the event, It does not specify who is eligible to receive this event.
    pub agent_id: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_permission_requested() {
        let message = r#"
        {
            "kind": "permission.requested",
            "agent_id": "agent_id",
            "data": {
                "agent_id": "agent_id",
                "session_id": "session_id",
                "request_id": "request_id",
                "tool_call_id": "tool_call_id",
                "tool_call": {"name": "test"},
                "options": []
            }
        }
        "#;
        let msg: EventMessage = serde_json::from_str(message).unwrap();
        assert_eq!(msg.agent_id, "agent_id");
        assert!(matches!(msg.event, Event::Builtin(BuiltinEvent::PermissionRequested(_))));
    }

    #[test]
    fn test_deserialize_task_created() {
        let message = r#"
        {
            "kind": "task.created",
            "agent_id": "agent_id",
            "data": {
                "task_id": "task_123",
                "title": "Test Task"
            }
        }
        "#;
        let msg: EventMessage = serde_json::from_str(message).unwrap();
        if let Event::Builtin(BuiltinEvent::TaskCreated(data)) = msg.event {
            assert_eq!(data.task_id, "task_123");
            assert_eq!(data.title, "Test Task");
        } else {
            panic!("Expected TaskCreated event");
        }
    }

    #[test]
    fn test_deserialize_relay_spawn_requested() {
        let message = r#"
        {
            "kind": "relay.spawn_requested",
            "agent_id": "system",
            "data": {
                "relay_id": "relay_123",
                "request_id": "req_123",
                "target_agent_id": "agent_456",
                "session_id": "session_789",
                "workdir": "/tmp",
                "command": "echo",
                "args": ["hello"],
                "env": {}
            }
        }
        "#;
        let msg: EventMessage = serde_json::from_str(message).unwrap();
        if let Event::Builtin(BuiltinEvent::RelaySpawnRequested(data)) = msg.event {
            assert_eq!(data.relay_id, "relay_123");
            assert_eq!(data.target_agent_id, "agent_456");
            assert_eq!(data.command, "echo");
        } else {
            panic!("Expected RelaySpawnRequested event");
        }
    }

    #[test]
    fn test_deserialize_custom_event() {
        let message = r#"
        {
            "kind": "custom.event",
            "agent_id": "agent_id",
            "data": {"foo": "bar"}
        }
        "#;
        let msg: EventMessage = serde_json::from_str(message).unwrap();
        if let Event::Custom { kind, data } = msg.event {
            assert_eq!(kind, "custom.event");
            assert_eq!(data, serde_json::json!({"foo": "bar"}));
        } else {
            panic!("Expected Custom event");
        }
    }

    #[test]
    fn test_serialize_builtin_event() {
        let event = BuiltinEvent::TaskCreated(TaskCreatedData {
            task_id: "task_123".to_string(),
            title: "Test".to_string(),
            description: None,
            parent_task_id: None,
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""kind":"task.created""#));
        assert!(json.contains(r#""task_id":"task_123""#));
    }

    #[test]
    fn test_permission_outcome() {
        let selected = PermissionOutcome::selected("allow");
        let json = serde_json::to_string(&selected).unwrap();
        assert!(json.contains(r#""selected""#));
        assert!(json.contains(r#""option_id":"allow""#));

        let cancelled = PermissionOutcome::cancelled();
        let json = serde_json::to_string(&cancelled).unwrap();
        assert!(json.contains(r#""cancelled":true"#));
    }
}
