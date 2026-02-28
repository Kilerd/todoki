/// Re-export event kinds from shared protocol
///
/// All event kind constants are defined in todoki-protocol for sharing
/// between server and relay. This module re-exports them for backward
/// compatibility.

/// Backward compatibility: EventKind struct with associated constants
/// Prefer using the module-level constants directly (e.g., `todoki_protocol::event_kind::TASK_CREATED`)
pub struct EventKind;

impl EventKind {
    // Task lifecycle
    pub const TASK_CREATED: &'static str = todoki_protocol::event_kind::TASK_CREATED;
    pub const TASK_STATUS_CHANGED: &'static str = todoki_protocol::event_kind::TASK_STATUS_CHANGED;
    pub const TASK_ASSIGNED: &'static str = todoki_protocol::event_kind::TASK_ASSIGNED;
    pub const TASK_COMPLETED: &'static str = todoki_protocol::event_kind::TASK_COMPLETED;
    pub const TASK_FAILED: &'static str = todoki_protocol::event_kind::TASK_FAILED;
    pub const TASK_ARCHIVED: &'static str = todoki_protocol::event_kind::TASK_ARCHIVED;

    // Agent lifecycle
    pub const AGENT_REGISTERED: &'static str = todoki_protocol::event_kind::AGENT_REGISTERED;
    pub const AGENT_STARTED: &'static str = todoki_protocol::event_kind::AGENT_STARTED;
    pub const AGENT_STOPPED: &'static str = todoki_protocol::event_kind::AGENT_STOPPED;
    pub const AGENT_OUTPUT: &'static str = todoki_protocol::event_kind::AGENT_OUTPUT;
    pub const AGENT_ERROR: &'static str = todoki_protocol::event_kind::AGENT_ERROR;

    // Agent collaboration
    pub const REQUIREMENT_ANALYZED: &'static str = todoki_protocol::event_kind::REQUIREMENT_ANALYZED;
    pub const BUSINESS_CONTEXT_READY: &'static str = todoki_protocol::event_kind::BUSINESS_CONTEXT_READY;
    pub const CODE_REVIEW_REQUESTED: &'static str = todoki_protocol::event_kind::CODE_REVIEW_REQUESTED;
    pub const QA_TEST_PASSED: &'static str = todoki_protocol::event_kind::QA_TEST_PASSED;
    pub const QA_TEST_FAILED: &'static str = todoki_protocol::event_kind::QA_TEST_FAILED;

    // Artifacts
    pub const ARTIFACT_CREATED: &'static str = todoki_protocol::event_kind::ARTIFACT_CREATED;
    pub const GITHUB_PR_OPENED: &'static str = todoki_protocol::event_kind::GITHUB_PR_OPENED;
    pub const GITHUB_PR_MERGED: &'static str = todoki_protocol::event_kind::GITHUB_PR_MERGED;

    // Agent Session Events
    pub const AGENT_SESSION_STARTED: &'static str = todoki_protocol::event_kind::AGENT_SESSION_STARTED;
    pub const AGENT_SESSION_EXITED: &'static str = todoki_protocol::event_kind::AGENT_SESSION_EXITED;

    // Permission
    pub const PERMISSION_REQUESTED: &'static str = todoki_protocol::event_kind::PERMISSION_REQUESTED;
    pub const PERMISSION_RESPONDED: &'static str = todoki_protocol::event_kind::PERMISSION_RESPONDED;
    pub const PERMISSION_APPROVED: &'static str = todoki_protocol::event_kind::PERMISSION_APPROVED;
    pub const PERMISSION_DENIED: &'static str = todoki_protocol::event_kind::PERMISSION_DENIED;

    // Relay Lifecycle
    pub const RELAY_UP: &'static str = todoki_protocol::event_kind::RELAY_UP;
    pub const RELAY_DOWN: &'static str = todoki_protocol::event_kind::RELAY_DOWN;

    // Relay Data Upload
    pub const RELAY_AGENT_OUTPUT: &'static str = todoki_protocol::event_kind::RELAY_AGENT_OUTPUT;
    pub const RELAY_AGENT_OUTPUT_BATCH: &'static str = todoki_protocol::event_kind::RELAY_AGENT_OUTPUT_BATCH;
    pub const RELAY_SESSION_STATUS: &'static str = todoki_protocol::event_kind::RELAY_SESSION_STATUS;
    pub const RELAY_PERMISSION_REQUEST: &'static str = todoki_protocol::event_kind::RELAY_PERMISSION_REQUEST;
    pub const RELAY_ARTIFACT: &'static str = todoki_protocol::event_kind::RELAY_ARTIFACT;
    pub const RELAY_PROMPT_COMPLETED: &'static str = todoki_protocol::event_kind::RELAY_PROMPT_COMPLETED;

    // Relay Commands
    pub const RELAY_SPAWN_REQUESTED: &'static str = todoki_protocol::event_kind::RELAY_SPAWN_REQUESTED;
    pub const RELAY_STOP_REQUESTED: &'static str = todoki_protocol::event_kind::RELAY_STOP_REQUESTED;
    pub const RELAY_INPUT_REQUESTED: &'static str = todoki_protocol::event_kind::RELAY_INPUT_REQUESTED;

    // Relay Responses
    pub const RELAY_SPAWN_COMPLETED: &'static str = todoki_protocol::event_kind::RELAY_SPAWN_COMPLETED;
    pub const RELAY_SPAWN_FAILED: &'static str = todoki_protocol::event_kind::RELAY_SPAWN_FAILED;
    pub const RELAY_STOP_COMPLETED: &'static str = todoki_protocol::event_kind::RELAY_STOP_COMPLETED;

    // System
    pub const RELAY_CONNECTED: &'static str = todoki_protocol::event_kind::RELAY_CONNECTED;
    pub const RELAY_DISCONNECTED: &'static str = todoki_protocol::event_kind::RELAY_DISCONNECTED;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kinds_format() {
        // Verify all event kinds follow the namespace.action format
        assert!(EventKind::TASK_CREATED.contains('.'));
        assert!(EventKind::AGENT_STARTED.contains('.'));
        assert!(EventKind::ARTIFACT_CREATED.contains('.'));
    }
}
