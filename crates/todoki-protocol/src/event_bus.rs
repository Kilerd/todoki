//! Structured event bus message definitions.
//!
//! All event types and their data structures are defined here for use by both
//! todoki-server and todoki-relay.

use std::collections::HashMap;

#[cfg(feature = "schematic")]
use gotcha::Schematic;
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

/// A permission option that users can select when responding to permission requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct PermissionOption {
    /// The type of permission option (e.g., "allow", "deny", "allow_always").
    pub kind: String,
    /// Human-readable display name for this option.
    pub name: String,
    /// Unique identifier for this option, used when responding to the permission request.
    pub option_id: String,
}

/// Information about a tool call that requires permission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct ToolCall {
    /// Human-readable title describing what the tool will do.
    pub title: String,
    /// The raw input parameters that will be passed to the tool.
    pub raw_input: Value,
    /// Unique identifier for this specific tool call instance.
    pub tool_call_id: Option<String>,
}

/// The selected option when a user approves a permission request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct SelectedOutcome {
    /// The option_id of the permission option that was selected.
    pub option_id: String,
}

/// The outcome of a permission request - either user selected an option or cancelled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
#[serde(untagged)]
pub enum PermissionOutcome {
    /// User selected a permission option.
    Selected {
        /// The selected outcome containing the chosen option_id.
        selected: SelectedOutcome,
    },
    /// User cancelled the permission request without selecting an option.
    Cancelled {
        /// Always true when the request was cancelled.
        cancelled: bool,
    },
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

/// Data for task.created event - emitted when a new task is created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskCreatedData {
    /// Unique identifier for the newly created task (UUID format).
    pub task_id: String,
    /// Short, descriptive title of the task.
    pub title: String,
    /// Detailed description of what the task involves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// If this is a subtask, the task_id of the parent task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<String>,
}

/// Data for task.status_changed event - emitted when a task's status transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskStatusChangedData {
    /// The task whose status changed.
    pub task_id: String,
    /// The previous status (e.g., "pending", "in_progress", "completed").
    pub old_status: String,
    /// The new status after the transition.
    pub new_status: String,
}

/// Data for task.assigned event - emitted when a task is assigned to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskAssignedData {
    /// The task being assigned.
    pub task_id: String,
    /// The agent_id of the agent that the task is assigned to.
    pub assigned_agent_id: String,
}

/// Data for task.completed event - emitted when a task finishes successfully.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskCompletedData {
    /// The task that was completed.
    pub task_id: String,
    /// Optional result data produced by completing the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// Data for task.failed event - emitted when a task fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskFailedData {
    /// The task that failed.
    pub task_id: String,
    /// Human-readable error message explaining why the task failed.
    pub error: String,
}

/// Data for task.archived event - emitted when a task is archived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct TaskArchivedData {
    /// The task that was archived.
    pub task_id: String,
}

// ============================================================================
// Agent Data Structures
// ============================================================================

/// Data for agent.registered event - emitted when an agent is registered with the system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentRegisteredData {
    /// Unique identifier for the agent (UUID format).
    pub agent_id: String,
    /// Type of agent (e.g., "claude", "gpt", "custom").
    pub agent_type: String,
    /// List of capabilities this agent supports (e.g., ["code_review", "testing"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

/// Data for agent.started event - emitted when an agent begins execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentStartedData {
    /// The agent that started.
    pub agent_id: String,
    /// Session ID for this execution instance.
    pub session_id: String,
}

/// Data for agent.stopped event - emitted when an agent stops execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentStoppedData {
    /// The agent that stopped.
    pub agent_id: String,
    /// Session ID that was terminated.
    pub session_id: String,
    /// Human-readable reason for stopping (e.g., "completed", "error", "user_cancelled").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Data for agent.output event - emitted for each line of agent output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentOutputData {
    /// The agent producing output.
    pub agent_id: String,
    /// Session ID for this execution.
    pub session_id: String,
    /// Output stream type: "stdout" or "stderr".
    pub stream: String,
    /// The output message content.
    pub message: String,
    /// Unix timestamp in milliseconds when the output was produced.
    pub ts: i64,
}

/// Data for agent.output_batch event - emitted for batched agent output.
/// More efficient than individual output events for high-volume output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentOutputBatchData {
    /// Session ID for this execution.
    pub session_id: String,
    /// Output stream type: "stdout" or "stderr".
    pub stream: String,
    /// Batch of output messages.
    pub messages: Vec<String>,
    /// Unix timestamp in milliseconds when the batch was created.
    pub ts: i64,
}

/// Data for agent.error event - emitted when an agent encounters an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentErrorData {
    /// The agent that encountered the error.
    pub agent_id: String,
    /// Session ID where the error occurred.
    pub session_id: String,
    /// Human-readable error message.
    pub error: String,
}

/// Data for agent.session_started event - emitted when a new agent session begins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentSessionStartedData {
    /// The agent starting a session.
    pub agent_id: String,
    /// Unique identifier for this session (UUID format).
    pub session_id: String,
}

/// Data for agent.session_exited event - emitted when an agent session terminates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct AgentSessionExitedData {
    /// The agent whose session exited.
    pub agent_id: String,
    /// Session ID that terminated.
    pub session_id: String,
    /// Process exit code (0 = success, non-zero = error).
    pub exit_code: Option<i32>,
}

// ============================================================================
// Agent Collaboration Data Structures
// ============================================================================

/// Data for agent.requirement_analyzed event - emitted when an agent completes requirement analysis.
/// Used for multi-agent collaboration workflows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RequirementAnalyzedData {
    /// The agent that performed the analysis.
    pub agent_id: String,
    /// The task whose requirements were analyzed.
    pub task_id: String,
    /// Structured analysis result (schema varies by agent type).
    pub analysis: Value,
}

/// Data for agent.business_context_ready event - emitted when business context is prepared.
/// Signals that downstream agents can begin their work with the provided context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct BusinessContextReadyData {
    /// The agent that prepared the context.
    pub agent_id: String,
    /// The task this context belongs to.
    pub task_id: String,
    /// Business context data (e.g., domain knowledge, constraints, stakeholder requirements).
    pub context: Value,
}

/// Data for agent.code_review_requested event - emitted when code review is needed.
/// Triggers code review agents to examine the changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct CodeReviewRequestedData {
    /// The agent requesting the review.
    pub agent_id: String,
    /// The task associated with the code changes.
    pub task_id: String,
    /// GitHub PR URL if the code is in a pull request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
}

/// Data for agent.qa_test_passed and agent.qa_test_failed events.
/// Indicates QA testing results for a task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct QaTestResultData {
    /// The QA agent that ran the tests.
    pub agent_id: String,
    /// The task being tested.
    pub task_id: String,
    /// Additional test result details (e.g., test counts, failure reasons, coverage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

// ============================================================================
// Artifact Data Structures
// ============================================================================

/// Data for artifact.created event - emitted when an agent produces an artifact.
/// Artifacts are tangible outputs like files, commits, PRs, documents, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct ArtifactCreatedData {
    /// Session ID that produced the artifact.
    pub session_id: String,
    /// Type of artifact (e.g., "file", "commit", "pr", "document", "test_report").
    pub artifact_type: String,
    /// Artifact-specific data (schema varies by artifact_type).
    pub data: Value,
}

/// Data for artifact.github_pr_opened and artifact.github_pr_merged events.
/// Tracks GitHub pull request lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct GithubPrData {
    /// The task that this PR is associated with.
    pub task_id: String,
    /// Full URL to the pull request (e.g., "https://github.com/owner/repo/pull/123").
    pub pr_url: String,
    /// Pull request number.
    pub pr_number: i64,
    /// Repository in "owner/repo" format.
    pub repo: String,
}

// ============================================================================
// Relay Data Structures
// ============================================================================
// Relays are edge nodes that execute agents locally and communicate with the server.
// Events flow bidirectionally:
// - Relay → Server: agent output, status updates, permission requests, artifacts
// - Server → Relay: spawn/stop commands, input forwarding

/// Data for relay.up and relay.down events - relay lifecycle events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayLifecycleData {
    /// Unique identifier for the relay (UUID format).
    pub relay_id: String,
}

/// Data for relay.agent_output event - single output line from an agent via relay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayAgentOutputData {
    /// The relay forwarding the output.
    pub relay_id: String,
    /// The agent producing the output.
    pub target_agent_id: String,
    /// Session ID for this execution.
    pub session_id: String,
    /// Output stream type: "stdout" or "stderr".
    pub stream: String,
    /// The output message content.
    pub message: String,
    /// Unix timestamp in milliseconds.
    pub ts: i64,
}

/// Data for relay.agent_output_batch event - batched output from an agent via relay.
/// Preferred over individual output events for efficiency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayAgentOutputBatchData {
    /// The relay forwarding the output.
    pub relay_id: String,
    /// The agent producing the output.
    pub target_agent_id: String,
    /// Session ID for this execution.
    pub session_id: String,
    /// Output stream type: "stdout" or "stderr".
    pub stream: String,
    /// Batch of output messages.
    pub messages: Vec<String>,
    /// Unix timestamp in milliseconds when the batch was created.
    pub ts: i64,
}

/// Data for relay.session_status event - reports agent session status changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelaySessionStatusData {
    /// The relay reporting the status.
    pub relay_id: String,
    /// Session ID being reported.
    pub session_id: String,
    /// Current status (e.g., "running", "exited", "error").
    pub status: String,
    /// Process exit code if the session has exited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// Data for relay.permission_request event - relay forwards permission request from agent.
/// The server should route this to the appropriate UI for user decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayPermissionRequestData {
    /// The relay forwarding the request.
    pub relay_id: String,
    /// The agent requesting permission.
    pub target_agent_id: String,
    /// Session ID where the request originated.
    pub session_id: String,
    /// Unique identifier for this permission request (for response correlation).
    pub request_id: String,
    /// Tool call ID for tracking.
    pub tool_call_id: String,
    /// Details of the tool call requiring permission.
    pub tool_call: ToolCall,
    /// Available permission options for the user to choose from.
    pub options: Vec<PermissionOption>,
}

/// Data for relay.artifact event - relay reports an artifact created by an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayArtifactData {
    /// The relay reporting the artifact.
    pub relay_id: String,
    /// The agent that created the artifact.
    pub target_agent_id: String,
    /// Session ID that produced the artifact.
    pub session_id: String,
    /// Type of artifact (e.g., "file", "commit", "pr").
    pub artifact_type: String,
    /// Additional artifact-specific fields.
    #[serde(flatten)]
    pub extra: Value,
}

/// Data for relay.prompt_completed event - indicates an agent prompt has finished.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayPromptCompletedData {
    /// The relay reporting completion.
    pub relay_id: String,
    /// Session ID where the prompt completed.
    pub session_id: String,
    /// Whether the prompt completed successfully.
    pub success: bool,
    /// Error message if success is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Data for relay.error event - relay reports an error condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayErrorData {
    /// The relay reporting the error.
    pub relay_id: String,
    /// Session ID where the error occurred.
    pub session_id: String,
    /// Human-readable error message.
    pub error: String,
}

/// Data for relay.spawn_requested event - server requests relay to spawn an agent process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelaySpawnRequestedData {
    /// Target relay that should spawn the process.
    pub relay_id: String,
    /// Unique request ID for response correlation.
    pub request_id: String,
    /// Agent ID for the spawned process.
    pub target_agent_id: String,
    /// Session ID to assign to this execution.
    pub session_id: String,
    /// Working directory for the process.
    pub workdir: String,
    /// Command to execute (e.g., "claude", "python").
    pub command: String,
    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set for the process.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Data for relay.stop_requested event - server requests relay to stop an agent session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayStopRequestedData {
    /// Target relay that should stop the session.
    pub relay_id: String,
    /// Session ID to stop.
    pub session_id: String,
}

/// Data for relay.input_requested event - server sends input to forward to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayInputRequestedData {
    /// Target relay that should forward the input.
    pub relay_id: String,
    /// Session ID to receive the input.
    pub session_id: String,
    /// Input text to send to the agent's stdin.
    pub input: String,
}

/// Data for relay.spawn_completed event - relay confirms successful agent spawn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelaySpawnCompletedData {
    /// The relay that completed the spawn.
    pub relay_id: String,
    /// Original request ID for correlation.
    pub request_id: String,
    /// Session ID that was spawned.
    pub session_id: String,
}

/// Data for relay.spawn_failed event - relay reports agent spawn failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelaySpawnFailedData {
    /// The relay that failed to spawn.
    pub relay_id: String,
    /// Original request ID for correlation.
    pub request_id: String,
    /// Session ID that failed to spawn.
    pub session_id: String,
    /// Human-readable error explaining why spawn failed.
    pub error: String,
}

/// Data for relay.stop_completed event - relay confirms session was stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct RelayStopCompletedData {
    /// The relay that stopped the session.
    pub relay_id: String,
    /// Session ID that was stopped.
    pub session_id: String,
}

// ============================================================================
// System Data Structures
// ============================================================================
// System events are internal infrastructure events, not directly related to tasks or agents.

/// Data for system.relay_connected and system.relay_disconnected events.
/// Tracks relay WebSocket connection lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct SystemRelayConnectionData {
    /// The relay that connected or disconnected.
    pub relay_id: String,
}

// ============================================================================
// Permission Event Data (for permission.requested/responded)
// ============================================================================
// These are higher-level permission events after relay routing has been resolved.
// Used by frontends to display permission dialogs and track responses.

/// Data for permission.requested event - emitted when an agent needs user approval.
/// This is the frontend-facing event (after relay routing is resolved).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct PermissionRequestedData {
    /// Session ID requesting permission.
    pub session_id: String,
    /// Unique identifier for this permission request (for response correlation).
    pub request_id: String,
    /// Tool call ID for tracking within the agent.
    pub tool_call_id: String,
    /// Details of the tool call requiring permission.
    pub tool_call: ToolCall,
    /// Available permission options for the user to choose from.
    pub options: Vec<PermissionOption>,
}

/// Data for permission.responded event - emitted when a user responds to a permission request.
/// Contains the routing information needed to deliver the response to the correct relay/agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
pub struct PermissionRespondedData {
    /// Target relay to deliver the response to.
    pub relay_id: String,
    /// Permission request ID being responded to.
    pub request_id: String,
    /// Session ID to receive the response.
    pub session_id: String,
    /// User's decision: either selected option or cancelled.
    pub outcome: PermissionOutcome,
}

// ============================================================================
// Builtin Event Enum
// ============================================================================

/// All builtin event types with their structured data.
///
/// Events are categorized by domain:
/// - **Task events**: Task lifecycle (created, assigned, completed, failed, etc.)
/// - **Agent events**: Agent lifecycle and output (started, stopped, output, error)
/// - **Artifact events**: Agent-produced artifacts (files, PRs, commits)
/// - **Permission events**: Permission request/response flow
/// - **Relay events**: Relay-to-server communication (output forwarding, status, commands)
/// - **System events**: Infrastructure events (relay connections)
///
/// The enum is serialized with `#[serde(tag = "kind", content = "data")]`,
/// producing JSON like: `{"kind": "task.created", "data": {...}}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
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

/// Event that can be either a builtin typed event or a custom untyped event.
/// Use builtin events for type safety; use custom events for extensibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(Schematic))]
#[serde(untagged)]
pub enum Event {
    /// A builtin event with strongly-typed data.
    Builtin(BuiltinEvent),
    /// A custom event for user-defined event types.
    Custom {
        /// Event kind string (e.g., "custom.my_event").
        kind: String,
        /// Arbitrary JSON data for the event.
        data: Value,
    },
}

/// Complete event bus message including the emitting agent.
/// This is the top-level structure for all events in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schematic", derive(gotcha::Schematic))]
pub struct EventMessage {
    /// The event payload (builtin or custom).
    #[serde(flatten)]
    pub event: Event,
    /// The agent_id of the agent that emitted this event.
    /// This identifies the sender, not the subject or recipient of the event.
    /// For example, in a relay.agent_output event, agent_id is the relay,
    /// while target_agent_id in the data identifies which agent produced the output.
    pub agent_id: String,
}

impl EventMessage {
    /// Extract kind and data for storage layer.
    ///
    /// This method is used when storing events in the database, where we need
    /// the flat (kind, data) representation rather than the typed enum.
    ///
    /// # Returns
    /// A tuple of (kind, data) where kind is the event type string (e.g., "task.created")
    /// and data is the JSON payload.
    pub fn into_parts(self) -> (String, Value) {
        self.event.into_parts()
    }
}

impl Event {
    /// Extract kind and data from the event.
    ///
    /// # Returns
    /// A tuple of (kind, data) where kind is the event type string and data is the JSON payload.
    pub fn into_parts(self) -> (String, Value) {
        match self {
            Event::Builtin(builtin) => builtin.into_parts(),
            Event::Custom { kind, data } => (kind, data),
        }
    }
}

impl BuiltinEvent {
    /// Extract kind string and data value from a builtin event.
    ///
    /// This uses serde serialization to get the tagged format, then extracts
    /// the "kind" and "data" fields. This ensures consistency with the
    /// serde serialization format.
    ///
    /// # Returns
    /// A tuple of (kind, data) where kind matches the `#[serde(rename = "...")]` value
    /// and data is the serialized event data.
    pub fn into_parts(self) -> (String, Value) {
        // Serialize to get the tagged format, then extract
        let value = serde_json::to_value(&self).unwrap_or_default();
        let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let data = value.get("data").cloned().unwrap_or(Value::Null);
        (kind, data)
    }
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
                "session_id": "session_id",
                "request_id": "request_id",
                "tool_call_id": "tool_call_id",
                "tool_call": {
                    "title": "Run bash command",
                    "raw_input": {"command": "ls -la"}
                },
                "options": [
                    {"kind": "allow_once", "name": "Allow Once", "option_id": "opt_1"},
                    {"kind": "deny", "name": "Deny", "option_id": "opt_2"}
                ]
            }
        }
        "#;
        let msg: EventMessage = serde_json::from_str(message).unwrap();
        assert_eq!(msg.agent_id, "agent_id");
        if let Event::Builtin(BuiltinEvent::PermissionRequested(data)) = msg.event {
            assert_eq!(data.session_id, "session_id");
            assert_eq!(data.tool_call.title, "Run bash command");
            assert_eq!(data.options.len(), 2);
            assert_eq!(data.options[0].kind, "allow_once");
        } else {
            panic!("Expected PermissionRequested event");
        }
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
