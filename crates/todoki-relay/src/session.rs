use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::RequestPermissionOutcome;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::acp::{spawn_acp_session, AcpHandle};
use crate::relay::RelayOutput;
use todoki_protocol::{SendInputParams, SpawnSessionParams, SpawnSessionResult};

/// Manages a single local agent session (subprocess).
/// Only one session can be active at a time.
pub struct SessionManager {
    active_session: Arc<Mutex<Option<ActiveSession>>>,
    output_tx: mpsc::Sender<RelayOutput>,
    safe_paths: Vec<String>,
}

struct ActiveSession {
    session_id: String,
    child: Child,
    acp_handle: AcpHandle,
    /// Sender to signal that the session should be terminated
    kill_tx: Option<oneshot::Sender<()>>,
}

impl SessionManager {
    pub fn new(output_tx: mpsc::Sender<RelayOutput>, safe_paths: Vec<String>) -> Self {
        Self {
            active_session: Arc::new(Mutex::new(None)),
            output_tx,
            safe_paths,
        }
    }

    /// Spawn a new session
    pub async fn spawn(&self, params: SpawnSessionParams) -> anyhow::Result<SpawnSessionResult> {
        // Single-task mode: only one session at a time
        {
            let session = self.active_session.lock().await;
            if session.is_some() {
                anyhow::bail!("relay busy: already running session");
            }
        }

        tracing::debug!(
            session_id = %params.session_id,
            agent_id = %params.agent_id,
            command = %params.command,
            workdir = %params.workdir,
            args = ?params.args,
            "spawn called"
        );

        // Validate workdir against safe paths
        if !self.is_path_safe(&params.workdir) {
            tracing::error!(workdir = %params.workdir, "workdir not in safe paths");
            anyhow::bail!("workdir not in safe paths: {}", params.workdir);
        }

        let workdir = expand_tilde(&params.workdir);
        if !Path::new(&workdir).exists() {
            tracing::error!(workdir = %workdir, "workdir does not exist");
            anyhow::bail!("workdir does not exist: {}", workdir);
        }

        tracing::debug!(
            command = %params.command,
            workdir = %workdir,
            args = ?params.args,
            setup_script = ?params.setup_script,
            "spawning child process"
        );

        // Run setup script if provided
        if let Some(setup_script) = &params.setup_script {
            let setup_path = format!("{}/.todoki-setup-{}.sh", workdir, params.session_id);
            tracing::debug!(setup_path = %setup_path, "writing setup script");

            if let Err(e) = std::fs::write(&setup_path, setup_script) {
                anyhow::bail!("failed to write setup script: {}", e);
            }

            let status = Command::new("bash")
                .arg(&setup_path)
                .current_dir(&workdir)
                .envs(std::env::vars())
                .status()
                .await;

            // Clean up script file
            let _ = std::fs::remove_file(&setup_path);

            match status {
                Ok(s) if s.success() => {
                    tracing::debug!("setup script completed successfully");
                }
                Ok(s) => {
                    anyhow::bail!("setup script failed with exit code: {:?}", s.code());
                }
                Err(e) => {
                    anyhow::bail!("failed to run setup script: {}", e);
                }
            }
        }

        let mut command = Command::new(&params.command);
        command
            .args(&params.args)
            .current_dir(&workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Inherit all environment variables from parent process
            .envs(std::env::vars());

        // Override with any custom env vars from params
        for (key, value) in &params.env {
            command.env(key, value);
        }

        tracing::debug!(
            home = std::env::var("HOME").ok(),
            path = std::env::var("PATH").ok().map(|p| p.chars().take(100).collect::<String>()),
            "environment variables for child process"
        );

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, command = %params.command, "failed to spawn child process");
                return Err(e.into());
            }
        };
        let pid = child.id().unwrap_or(0);
        tracing::info!(pid = pid, command = %params.command, "child process spawned");

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();

        // Initialize ACP session
        tracing::debug!(session_id = %params.session_id, "initializing ACP session");
        let stdout = stdout.ok_or_else(|| anyhow::anyhow!("no stdout for ACP"))?;
        let stdin = stdin.ok_or_else(|| anyhow::anyhow!("no stdin for ACP"))?;

        // Spawn stderr reader to capture agent errors
        if let Some(stderr) = stderr {
            let session_id_for_stderr = params.session_id.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::warn!(
                        session_id = %session_id_for_stderr,
                        line = %line,
                        "agent stderr"
                    );
                    eprintln!("[STDERR] session={} {}", session_id_for_stderr, line);
                }
            });
        }

        tracing::debug!(session_id = %params.session_id, "calling spawn_acp_session");
        let acp_handle = match spawn_acp_session(
            self.output_tx.clone(),
            params.agent_id.clone(),
            params.session_id.clone(),
            workdir.clone(),
            stdout,
            stdin,
        )
        .await
        {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(
                    session_id = %params.session_id,
                    error = %e,
                    "failed to initialize ACP session"
                );
                return Err(e);
            }
        };

        tracing::info!(
            session_id = %params.session_id,
            acp_session_id = %acp_handle.acp_session_id,
            "ACP session initialized successfully"
        );

        // Create kill channel for termination signaling
        let (kill_tx, kill_rx) = oneshot::channel();

        let session = ActiveSession {
            session_id: params.session_id.clone(),
            child,
            acp_handle,
            kill_tx: Some(kill_tx),
        };

        // Store session
        {
            let mut active = self.active_session.lock().await;
            *active = Some(session);
        }

        // Spawn exit watcher
        self.spawn_exit_watcher(params.session_id.clone(), kill_rx);

        tracing::info!(
            session_id = %params.session_id,
            pid = pid,
            "spawn completed successfully"
        );
        Ok(SpawnSessionResult { pid })
    }

    /// Send input to a session.
    /// The session remains active until the process exits (handled by exit_watcher).
    pub async fn send_input(&self, params: SendInputParams) -> anyhow::Result<()> {
        tracing::debug!(
            session_id = %params.session_id,
            input_len = params.input.len(),
            "send_input called"
        );

        // Get the acp_handle without removing the session
        // Session cleanup is handled by exit_watcher when the process exits
        let acp_handle = {
            let active = self.active_session.lock().await;
            let session = active
                .as_ref()
                .filter(|s| s.session_id == params.session_id)
                .ok_or_else(|| anyhow::anyhow!("session not found: {}", params.session_id))?;
            session.acp_handle.clone()
        };

        tracing::debug!(
            session_id = %params.session_id,
            acp_session_id = %acp_handle.acp_session_id,
            "forwarding input to ACP"
        );

        let done_rx = acp_handle.prompt(params.input).await?;
        tracing::debug!(session_id = %params.session_id, "input sent to ACP");

        // Spawn a task to wait for this specific prompt completion and then terminate the process
        let session_id = params.session_id.clone();
        let active_session = self.active_session.clone();
        tokio::spawn(async move {
            tracing::debug!(session_id = %session_id, "waiting for prompt completion");

            // Wait for this specific prompt to complete
            if done_rx.await.is_err() {
                tracing::debug!(
                    session_id = %session_id,
                    "prompt done channel dropped, prompt may have been cancelled"
                );
                return;
            }

            tracing::info!(
                session_id = %session_id,
                "prompt completed, terminating agent process"
            );

            // Signal the exit watcher to kill the process
            let mut active = active_session.lock().await;
            if let Some(session) = active.as_mut() {
                if session.session_id == session_id {
                    if let Some(kill_tx) = session.kill_tx.take() {
                        let _ = kill_tx.send(());
                    }
                }
            }
        });

        Ok(())
    }

    /// Respond to a permission request
    pub async fn respond_permission(
        &self,
        session_id: &str,
        request_id: String,
        outcome: RequestPermissionOutcome,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            session_id = %session_id,
            request_id = %request_id,
            "respond_permission: acquiring session lock"
        );

        // Get the acp_handle while holding the lock, then release the lock
        // before doing async operations to avoid blocking other session operations
        let acp_handle = {
            let active = self.active_session.lock().await;
            let session = active
                .as_ref()
                .filter(|s| s.session_id == session_id)
                .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;
            session.acp_handle.clone()
        };

        tracing::debug!(
            session_id = %session_id,
            request_id = %request_id,
            "respond_permission: lock released, sending to ACP"
        );

        acp_handle.respond_permission(request_id.clone(), outcome).await?;

        tracing::debug!(
            session_id = %session_id,
            request_id = %request_id,
            "respond_permission: sent to ACP successfully"
        );

        Ok(())
    }

    /// Cancel current operation in a session
    pub async fn cancel(&self, session_id: &str) -> anyhow::Result<()> {
        let active = self.active_session.lock().await;
        let session = active
            .as_ref()
            .filter(|s| s.session_id == session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        session.acp_handle.cancel().await?;
        Ok(())
    }

    /// Stop a session by signaling the kill channel
    pub async fn stop(&self, session_id: &str) -> anyhow::Result<()> {
        let mut active = self.active_session.lock().await;
        if let Some(session) = active.as_mut() {
            if session.session_id == session_id {
                if let Some(kill_tx) = session.kill_tx.take() {
                    let _ = kill_tx.send(());
                }
            }
        }
        Ok(())
    }

    /// Stop all sessions (on disconnect)
    pub async fn stop_all(&self) {
        let mut active = self.active_session.lock().await;
        if let Some(session) = active.as_mut() {
            if let Some(kill_tx) = session.kill_tx.take() {
                let _ = kill_tx.send(());
            }
        }
    }

    fn spawn_exit_watcher(&self, session_id: String, kill_rx: oneshot::Receiver<()>) {
        let output_tx = self.output_tx.clone();
        let active_session = self.active_session.clone();

        tracing::debug!(session_id = %session_id, "spawning exit watcher");
        tokio::spawn(async move {
            tracing::debug!(session_id = %session_id, "exit watcher waiting for kill signal");

            // Wait for kill signal
            let _ = kill_rx.await;

            tracing::debug!(session_id = %session_id, "kill signal received, terminating process");

            // Take the session and kill the process
            let exit_status = {
                let mut active = active_session.lock().await;
                if let Some(mut session) = active.take() {
                    if session.session_id == session_id {
                        // Kill the process
                        let _ = session.child.kill().await;
                        // Wait for it to exit
                        session.child.wait().await.ok()
                    } else {
                        // Put it back if it's not our session (shouldn't happen)
                        *active = Some(session);
                        None
                    }
                } else {
                    None
                }
            };

            let (status, exit_code) = match &exit_status {
                Some(s) if s.success() => ("completed", s.code()),
                Some(s) => ("failed", s.code()),
                None => ("failed", None),
            };

            tracing::info!(
                session_id = %session_id,
                status = %status,
                exit_code = ?exit_code,
                "process exited"
            );

            // Notify server via Event Bus
            let msg = RelayOutput::EmitEvent {
                kind: "relay.session_status".to_string(),
                data: serde_json::json!({
                    "session_id": session_id,
                    "status": status,
                    "exit_code": exit_code,
                }),
            };

            let _ = output_tx.send(msg).await;
        });
    }

    pub(crate) fn is_path_safe(&self, path: &str) -> bool {
        if self.safe_paths.is_empty() {
            return true; // No restrictions if no safe paths configured
        }

        let target = normalize_path(&expand_tilde(path));
        for safe_path in &self.safe_paths {
            let allowed = normalize_path(&expand_tilde(safe_path));
            if target == allowed || target.starts_with(&format!("{}/", allowed)) {
                return true;
            }
        }
        false
    }
}

fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| path.to_string());
    }
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
    }
    path.to_string()
}

fn normalize_path(path: &str) -> String {
    let mut parts = Vec::new();
    for comp in std::path::Path::new(path).components() {
        match comp {
            std::path::Component::RootDir => parts.clear(),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(seg) => {
                parts.push(seg.to_string_lossy().to_string());
            }
            _ => {}
        }
    }
    format!("/{}", parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_home() {
        // Test ~ alone
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/test".to_string());
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn test_expand_tilde_with_path() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/test".to_string());
        assert_eq!(expand_tilde("~/Projects"), format!("{}/Projects", home));
        assert_eq!(
            expand_tilde("~/Projects/foo/bar"),
            format!("{}/Projects/foo/bar", home)
        );
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
        assert_eq!(expand_tilde(""), "");
    }

    #[test]
    fn test_normalize_path_basic() {
        assert_eq!(normalize_path("/foo/bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/bar/"), "/foo/bar");
    }

    #[test]
    fn test_normalize_path_with_dots() {
        assert_eq!(normalize_path("/foo/./bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/../bar"), "/bar");
        assert_eq!(normalize_path("/foo/bar/../baz"), "/foo/baz");
    }

    #[test]
    fn test_normalize_path_multiple_parent_dirs() {
        assert_eq!(normalize_path("/foo/bar/baz/../../qux"), "/foo/qux");
        assert_eq!(normalize_path("/foo/../../bar"), "/bar");
    }

    #[test]
    fn test_is_path_safe_empty_safe_paths() {
        let (manager, _rx) = {
            let (tx, rx) = tokio::sync::mpsc::channel::<RelayOutput>(1);
            (SessionManager::new(tx, vec![]), rx)
        };
        // Empty safe paths means no restrictions
        assert!(manager.is_path_safe("/any/path"));
        assert!(manager.is_path_safe("~/anywhere"));
    }

    #[test]
    fn test_is_path_safe_allowed() {
        let (manager, _rx) = {
            let (tx, rx) = tokio::sync::mpsc::channel::<RelayOutput>(1);
            (SessionManager::new(tx, vec!["/allowed".to_string()]), rx)
        };
        assert!(manager.is_path_safe("/allowed"));
        assert!(manager.is_path_safe("/allowed/sub/path"));
    }

    #[test]
    fn test_is_path_safe_disallowed() {
        let (manager, _rx) = {
            let (tx, rx) = tokio::sync::mpsc::channel::<RelayOutput>(1);
            (SessionManager::new(tx, vec!["/allowed".to_string()]), rx)
        };
        assert!(!manager.is_path_safe("/not-allowed"));
        assert!(!manager.is_path_safe("/allowed-but-not-really")); // Not a subpath
    }

    #[test]
    fn test_is_path_safe_with_tilde() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/test".to_string());
        let (manager, _rx) = {
            let (tx, rx) = tokio::sync::mpsc::channel::<RelayOutput>(1);
            (SessionManager::new(tx, vec!["~".to_string()]), rx)
        };
        assert!(manager.is_path_safe(&home));
        assert!(manager.is_path_safe(&format!("{}/Projects", home)));
        assert!(manager.is_path_safe("~/Projects"));
    }

    #[test]
    fn test_is_path_safe_traversal_attack() {
        let (manager, _rx) = {
            let (tx, rx) = tokio::sync::mpsc::channel::<RelayOutput>(1);
            (SessionManager::new(tx, vec!["/allowed".to_string()]), rx)
        };
        // Path traversal should be normalized and rejected
        assert!(!manager.is_path_safe("/allowed/../etc/passwd"));
        assert!(!manager.is_path_safe("/allowed/sub/../../etc"));
    }
}
