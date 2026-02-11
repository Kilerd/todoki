use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::RequestPermissionOutcome;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc};

use crate::acp::{spawn_acp_session, AcpHandle};
use crate::protocol::{RelayToServer, SendInputParams, SpawnSessionParams, SpawnSessionResult};

/// Manages local agent sessions (subprocesses)
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    output_tx: mpsc::Sender<RelayToServer>,
    safe_paths: Vec<String>,
}

struct Session {
    agent_id: String,
    session_id: String,
    child: Arc<Mutex<Option<Child>>>,
    acp_handle: AcpHandle,
}

impl SessionManager {
    pub fn new(output_tx: mpsc::Sender<RelayToServer>, safe_paths: Vec<String>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            output_tx,
            safe_paths,
        }
    }

    /// Spawn a new session
    pub async fn spawn(&self, params: SpawnSessionParams) -> anyhow::Result<SpawnSessionResult> {
        // Single-task mode: only one session at a time
        {
            let sessions = self.sessions.lock().await;
            if !sessions.is_empty() {
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

        let child_arc = Arc::new(Mutex::new(Some(child)));

        // Initialize ACP session
        tracing::debug!(session_id = %params.session_id, "initializing ACP session");
        let stdout = stdout.ok_or_else(|| anyhow::anyhow!("no stdout for ACP"))?;
        let stdin = stdin.ok_or_else(|| anyhow::anyhow!("no stdin for ACP"))?;

        // stderr is not used in ACP mode
        drop(stderr);

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

        let session = Session {
            agent_id: params.agent_id.clone(),
            session_id: params.session_id.clone(),
            child: child_arc.clone(),
            acp_handle,
        };

        // Store session
        {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(params.session_id.clone(), session);
        }

        // Spawn exit watcher
        self.spawn_exit_watcher(params.agent_id.clone(), params.session_id.clone(), child_arc);

        tracing::info!(
            session_id = %params.session_id,
            pid = pid,
            "spawn completed successfully"
        );
        Ok(SpawnSessionResult { pid })
    }

    /// Send input to a session
    pub async fn send_input(&self, params: SendInputParams) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(&params.session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", params.session_id))?;

        session.acp_handle.prompt(params.input).await?;
        Ok(())
    }

    /// Respond to a permission request
    pub async fn respond_permission(
        &self,
        session_id: &str,
        request_id: String,
        outcome: RequestPermissionOutcome,
    ) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        session.acp_handle.respond_permission(request_id, outcome).await?;
        Ok(())
    }

    /// Cancel current operation in a session
    pub async fn cancel(&self, session_id: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        session.acp_handle.cancel().await?;
        Ok(())
    }

    /// Stop a session
    pub async fn stop(&self, session_id: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            let mut child_guard = session.child.lock().await;
            if let Some(child) = child_guard.as_mut() {
                let _ = child.kill().await;
            }
        }
        Ok(())
    }

    /// Stop all sessions (on disconnect)
    pub async fn stop_all(&self) {
        let sessions = self.sessions.lock().await;
        for session in sessions.values() {
            let mut child_guard = session.child.lock().await;
            if let Some(child) = child_guard.as_mut() {
                let _ = child.kill().await;
            }
        }
    }

    fn spawn_exit_watcher(
        &self,
        _agent_id: String,
        session_id: String,
        child: Arc<Mutex<Option<Child>>>,
    ) {
        let output_tx = self.output_tx.clone();
        let sessions = self.sessions.clone();
        let sid = session_id.clone();

        tracing::debug!(session_id = %sid, "spawning exit watcher");
        tokio::spawn(async move {
            tracing::debug!(session_id = %session_id, "exit watcher waiting for process");
            let exit_status = {
                let mut child_guard = child.lock().await;
                if let Some(child) = child_guard.as_mut() {
                    child.wait().await.ok()
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

            // Remove from sessions
            {
                let mut sessions = sessions.lock().await;
                sessions.remove(&session_id);
            }

            // Notify server
            let msg = RelayToServer::SessionStatus {
                session_id,
                status: status.to_string(),
                exit_code,
            };

            let _ = output_tx.send(msg).await;
        });
    }

    fn is_path_safe(&self, path: &str) -> bool {
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
