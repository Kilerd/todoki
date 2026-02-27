/// Standard event kinds (namespace-prefixed)
///
/// This module defines the event taxonomy for the system.
/// Event kinds follow the format: `<category>.<action>`
///
/// Categories:
/// - task: Task lifecycle events
/// - agent: Agent lifecycle and collaboration events
/// - artifact: External artifacts (PRs, issues, etc.)
/// - permission: Permission request/response events
/// - system: System-level events

pub struct EventKind;

impl EventKind {
    // ========================================================================
    // Task lifecycle
    // ========================================================================
    pub const TASK_CREATED: &'static str = "task.created";
    pub const TASK_STATUS_CHANGED: &'static str = "task.status_changed";
    pub const TASK_ASSIGNED: &'static str = "task.assigned";
    pub const TASK_COMPLETED: &'static str = "task.completed";
    pub const TASK_FAILED: &'static str = "task.failed";
    pub const TASK_ARCHIVED: &'static str = "task.archived";

    // ========================================================================
    // Agent lifecycle
    // ========================================================================
    pub const AGENT_REGISTERED: &'static str = "agent.registered";
    pub const AGENT_STARTED: &'static str = "agent.started";
    pub const AGENT_STOPPED: &'static str = "agent.stopped";
    pub const AGENT_OUTPUT: &'static str = "agent.output";
    pub const AGENT_ERROR: &'static str = "agent.error";

    // ========================================================================
    // Agent collaboration (PM → BA → Coding → QA pipeline)
    // ========================================================================
    pub const REQUIREMENT_ANALYZED: &'static str = "agent.requirement_analyzed";
    pub const BUSINESS_CONTEXT_READY: &'static str = "agent.business_context_ready";
    pub const CODE_REVIEW_REQUESTED: &'static str = "agent.code_review_requested";
    pub const QA_TEST_PASSED: &'static str = "agent.qa_test_passed";
    pub const QA_TEST_FAILED: &'static str = "agent.qa_test_failed";

    // ========================================================================
    // Artifacts
    // ========================================================================
    pub const ARTIFACT_CREATED: &'static str = "artifact.created";
    pub const GITHUB_PR_OPENED: &'static str = "artifact.github_pr_opened";
    pub const GITHUB_PR_MERGED: &'static str = "artifact.github_pr_merged";

    // ========================================================================
    // Permission
    // ========================================================================
    pub const PERMISSION_REQUESTED: &'static str = "permission.requested";
    pub const PERMISSION_APPROVED: &'static str = "permission.approved";
    pub const PERMISSION_DENIED: &'static str = "permission.denied";

    // ========================================================================
    // System
    // ========================================================================
    pub const RELAY_CONNECTED: &'static str = "system.relay_connected";
    pub const RELAY_DISCONNECTED: &'static str = "system.relay_disconnected";
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
