//! Integration tests for todoki-relay SessionManager
//!
//! These tests verify the core session management functionality:
//! - Spawning sessions with mock-agent
//! - Sending input to sessions
//! - Permission request/response flow
//! - Path validation

use std::time::Duration;

use agent_client_protocol::RequestPermissionOutcome;
use tokio::sync::mpsc;

use todoki_protocol::{SendInputParams, SpawnSessionParams};
use todoki_relay::relay::RelayOutput;
use todoki_relay::session::SessionManager;

/// Get the path to the mock-agent binary
fn mock_agent_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // The binary should be in target/debug after building
    format!(
        "{}/../../target/debug/mock-agent",
        manifest_dir
    )
}

/// Create a SessionManager with a receiver for output messages
fn create_session_manager(safe_paths: Vec<String>) -> (SessionManager, mpsc::Receiver<RelayOutput>) {
    let (tx, rx) = mpsc::channel(64);
    let manager = SessionManager::new(tx, safe_paths, String::new(), String::new());
    (manager, rx)
}

/// Check if a RelayOutput message matches session status for the given session
fn is_session_status_for(msg: &RelayOutput, session_id: &str) -> bool {
    let RelayOutput::EmitEvent { kind, data } = msg;
    if kind == "relay.session_status" {
        if let Some(sid) = data.get("session_id").and_then(|v| v.as_str()) {
            return sid == session_id;
        }
    }
    false
}

/// Check if a RelayOutput message matches prompt completed for the given session
fn is_prompt_completed_for(msg: &RelayOutput, session_id: &str) -> Option<bool> {
    let RelayOutput::EmitEvent { kind, data } = msg;
    if kind == "relay.prompt_completed" {
        if let Some(sid) = data.get("session_id").and_then(|v| v.as_str()) {
            if sid == session_id {
                return data.get("success").and_then(|v| v.as_bool());
            }
        }
    }
    None
}

#[tokio::test]
async fn test_spawn_session() {
    // First ensure mock-agent is built
    let status = tokio::process::Command::new("cargo")
        .args(["build", "-p", "mock-agent"])
        .status()
        .await
        .expect("failed to build mock-agent");
    assert!(status.success(), "mock-agent build failed");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let workdir = temp_dir.path().to_string_lossy().to_string();

    let (manager, mut rx) = create_session_manager(vec![workdir.clone()]);

    let params = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-1".to_string(),
        command: mock_agent_path(),
        args: vec![],
        workdir: workdir.clone(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    let result = manager.spawn(params).await;
    assert!(result.is_ok(), "spawn failed: {:?}", result.err());

    let spawn_result = result.unwrap();
    assert!(spawn_result.pid > 0, "expected valid pid");

    // Clean up - stop the session
    manager.stop("test-session-1").await.ok();

    // Wait for exit status message
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(msg) = rx.recv().await {
            if is_session_status_for(&msg, "test-session-1") {
                return;
            }
        }
    })
    .await
    .ok();
}

#[tokio::test]
async fn test_send_input_simple() {
    // Build mock-agent
    let status = tokio::process::Command::new("cargo")
        .args(["build", "-p", "mock-agent"])
        .status()
        .await
        .expect("failed to build mock-agent");
    assert!(status.success(), "mock-agent build failed");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let workdir = temp_dir.path().to_string_lossy().to_string();

    let (manager, mut rx) = create_session_manager(vec![workdir.clone()]);

    // Spawn session
    let params = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-2".to_string(),
        command: mock_agent_path(),
        args: vec![],
        workdir: workdir.clone(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    manager.spawn(params).await.expect("spawn failed");

    // Send input
    let input_params = SendInputParams {
        session_id: "test-session-2".to_string(),
        input: "Hello, world!".to_string(),
    };

    let result = manager.send_input(input_params).await;
    assert!(result.is_ok(), "send_input failed: {:?}", result.err());

    // Wait for prompt completed message
    let completed = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(msg) = rx.recv().await {
            if let Some(success) = is_prompt_completed_for(&msg, "test-session-2") {
                return success;
            }
        }
        false
    })
    .await;

    assert!(completed.unwrap_or(false), "prompt did not complete successfully");
}

#[tokio::test]
async fn test_permission_flow() {
    // This test verifies that respond_permission can be called while prompt is in progress.
    // Previously there was a bug where send_input removed the session from the
    // HashMap before respond_permission could be called.
    //
    // Note: Since mock-agent completes prompts immediately (doesn't actually wait for
    // permission responses), we test the session lookup timing differently:
    // We spawn a session but don't send input, then verify respond_permission can find it.

    // Build mock-agent
    let status = tokio::process::Command::new("cargo")
        .args(["build", "-p", "mock-agent"])
        .status()
        .await
        .expect("failed to build mock-agent");
    assert!(status.success(), "mock-agent build failed");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let workdir = temp_dir.path().to_string_lossy().to_string();

    let (manager, _rx) = create_session_manager(vec![workdir.clone()]);

    // Spawn session
    let params = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-perm".to_string(),
        command: mock_agent_path(),
        args: vec![],
        workdir: workdir.clone(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    manager.spawn(params).await.expect("spawn failed");

    // Without sending input (prompt not started yet), verify respond_permission can find the session
    let perm_result = manager
        .respond_permission(
            "test-session-perm",
            "fake-request-id".to_string(),
            RequestPermissionOutcome::Cancelled,
        )
        .await;

    // This should succeed (session is found) - the response will be sent to ACP
    // even though there's no matching pending request
    assert!(
        perm_result.is_ok(),
        "respond_permission should succeed when session exists: {:?}",
        perm_result.err()
    );

    // Clean up
    manager.stop("test-session-perm").await.ok();

    // Wait for cleanup to complete
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_path_validation() {
    let (manager, _rx) = create_session_manager(vec!["/allowed/path".to_string()]);

    // Try to spawn with disallowed path
    let params = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-bad-path".to_string(),
        command: "echo".to_string(),
        args: vec!["hello".to_string()],
        workdir: "/not/allowed/path".to_string(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    let result = manager.spawn(params).await;
    assert!(result.is_err(), "spawn should fail for disallowed path");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not in safe paths"),
        "expected 'not in safe paths' error, got: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_spawn_duplicate_session() {
    // Build mock-agent
    let status = tokio::process::Command::new("cargo")
        .args(["build", "-p", "mock-agent"])
        .status()
        .await
        .expect("failed to build mock-agent");
    assert!(status.success(), "mock-agent build failed");

    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let workdir = temp_dir.path().to_string_lossy().to_string();

    let (manager, _rx) = create_session_manager(vec![workdir.clone()]);

    // Spawn first session
    let params1 = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-dup".to_string(),
        command: mock_agent_path(),
        args: vec![],
        workdir: workdir.clone(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    manager.spawn(params1).await.expect("first spawn failed");

    // Try to spawn second session - should fail (single-session mode)
    let params2 = SpawnSessionParams {
        agent_id: "test-agent".to_string(),
        session_id: "test-session-dup-2".to_string(),
        command: mock_agent_path(),
        args: vec![],
        workdir: workdir.clone(),
        env: std::collections::HashMap::new(),
        setup_script: None,
        task_id: None,
    };

    let result = manager.spawn(params2).await;
    assert!(result.is_err(), "second spawn should fail in single-session mode");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("already running"),
        "expected 'already running' error, got: {}",
        err_msg
    );

    // Clean up
    manager.stop_all().await;
}
