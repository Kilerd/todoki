use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::RequestPermissionOutcome;
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc};

use crate::acp::{spawn_acp_session, AcpHandle};
use crate::protocol::{RelayToServer, SendInputParams, SpawnSessionParams, SpawnSessionResult};

/// Manages local agent sessions (subprocesses)
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    output_tx: mpsc::Sender<RelayToServer>,
    safe_paths: Vec<String>,
}

/// Input mode for a session
enum SessionInput {
    /// Standard stdin for non-ACP processes
    Stdin(Arc<Mutex<Option<ChildStdin>>>),
    /// ACP handle for Claude-like agents
    Acp(AcpHandle),
}

struct Session {
    agent_id: String,
    session_id: String,
    child: Arc<Mutex<Option<Child>>>,
    input: SessionInput,
}

impl SessionManager {
    pub fn new(output_tx: mpsc::Sender<RelayToServer>, safe_paths: Vec<String>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            output_tx,
            safe_paths,
        }
    }

    /// Check if command should use ACP mode
    /// Note: Claude CLI does NOT support ACP protocol.
    /// ACP is implemented by specialized programs like `agenthub-codex-acp`.
    fn is_acp_command(command: &str) -> bool {
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command);
        // Only these specific programs implement ACP protocol
        matches!(cmd_name, "agenthub-codex-acp" | "codex-acp")
    }

    /// Check if command needs stdin closed immediately after spawn
    /// Some commands (like `claude --print`) wait for EOF before processing
    fn needs_immediate_stdin_close(command: &str, args: &[String]) -> bool {
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command);

        // claude --print needs EOF to start processing
        if cmd_name == "claude" && args.iter().any(|a| a == "--print" || a == "-p") {
            // Only close stdin if there's a prompt in args (one-shot mode)
            // If using stream-json input, we need to keep stdin open
            let has_stream_input = args.iter().any(|a| a.contains("stream-json"));
            return !has_stream_input;
        }

        false
    }

    /// Spawn a new session
    pub async fn spawn(&self, params: SpawnSessionParams) -> anyhow::Result<SpawnSessionResult> {
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

        let use_acp = Self::is_acp_command(&params.command);
        tracing::debug!(command = %params.command, use_acp = use_acp, "determined execution mode");

        tracing::debug!(
            command = %params.command,
            workdir = %workdir,
            args = ?params.args,
            "spawning child process"
        );

        let mut command = Command::new(&params.command);
        command
            .current_dir(&workdir)
            .args(&params.args)
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

        // Create session with appropriate input mode
        let session = if use_acp {
            tracing::debug!(session_id = %params.session_id, "initializing ACP session");
            // ACP mode: use agent-client-protocol
            let stdout = stdout.ok_or_else(|| anyhow::anyhow!("no stdout for ACP"))?;
            let stdin = stdin.ok_or_else(|| anyhow::anyhow!("no stdin for ACP"))?;

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

            Session {
                agent_id: params.agent_id.clone(),
                session_id: params.session_id.clone(),
                child: child_arc.clone(),
                input: SessionInput::Acp(acp_handle),
            }
        } else {
            // Standard stdin mode
            // Check if this is a one-shot command that needs stdin closed immediately
            // (e.g., `claude --print "prompt"` waits for EOF before processing)
            let needs_immediate_eof = Self::needs_immediate_stdin_close(&params.command, &params.args);

            let stdin = if needs_immediate_eof {
                tracing::debug!(session_id = %params.session_id, "closing stdin immediately for one-shot command");
                // Drop stdin to send EOF
                drop(stdin);
                None
            } else {
                stdin
            };

            let stdin = Arc::new(Mutex::new(stdin));

            // Spawn output readers for non-ACP mode
            if let Some(stdout) = stdout {
                self.spawn_output_reader(
                    params.agent_id.clone(),
                    params.session_id.clone(),
                    "stdout",
                    stdout,
                );
            }

            if let Some(stderr) = stderr {
                self.spawn_output_reader(
                    params.agent_id.clone(),
                    params.session_id.clone(),
                    "stderr",
                    stderr,
                );
            }

            Session {
                agent_id: params.agent_id.clone(),
                session_id: params.session_id.clone(),
                child: child_arc.clone(),
                input: SessionInput::Stdin(stdin),
            }
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

        match &session.input {
            SessionInput::Acp(handle) => {
                // ACP mode: send as prompt
                handle.prompt(params.input).await?;
                Ok(())
            }
            SessionInput::Stdin(stdin) => {
                // Standard stdin mode
                let mut stdin_guard = stdin.lock().await;
                if let Some(stdin_ref) = stdin_guard.as_mut() {
                    stdin_ref.write_all(params.input.as_bytes()).await?;
                    stdin_ref.flush().await?;

                    // If input ends with EOF marker, close stdin
                    if params.input.ends_with("\x04") || params.input.ends_with("<<EOF>>") {
                        tracing::debug!(session_id = %params.session_id, "closing stdin (EOF marker)");
                        *stdin_guard = None; // Drop the stdin to send EOF
                    }
                    Ok(())
                } else {
                    anyhow::bail!("session stdin closed")
                }
            }
        }
    }

    /// Close stdin to send EOF signal (for commands like `claude --print`)
    pub async fn close_stdin(&self, session_id: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        match &session.input {
            SessionInput::Stdin(stdin) => {
                let mut stdin_guard = stdin.lock().await;
                *stdin_guard = None; // Drop to send EOF
                tracing::debug!(session_id = %session_id, "stdin closed");
                Ok(())
            }
            SessionInput::Acp(_) => {
                // ACP mode doesn't need explicit EOF
                Ok(())
            }
        }
    }

    /// Respond to a permission request (ACP mode only)
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

        match &session.input {
            SessionInput::Acp(handle) => {
                handle.respond_permission(request_id, outcome).await?;
                Ok(())
            }
            SessionInput::Stdin(_) => {
                anyhow::bail!("session is not in ACP mode")
            }
        }
    }

    /// Cancel current operation in a session (ACP mode only)
    pub async fn cancel(&self, session_id: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        match &session.input {
            SessionInput::Acp(handle) => {
                handle.cancel().await?;
                Ok(())
            }
            SessionInput::Stdin(_) => {
                // For non-ACP sessions, cancellation is not supported via this method
                // Use stop() to kill the process instead
                Ok(())
            }
        }
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

    fn spawn_output_reader<R>(
        &self,
        agent_id: String,
        session_id: String,
        stream: &'static str,
        reader: R,
    ) where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let output_tx = self.output_tx.clone();
        let sid = session_id.clone();
        tracing::debug!(session_id = %sid, stream = %stream, "spawning output reader");
        tokio::spawn(async move {
            tracing::debug!(session_id = %session_id, stream = %stream, "output reader started");
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(
                    session_id = %session_id,
                    stream = %stream,
                    line_len = line.len(),
                    "read line from process"
                );
                let seq = Utc::now().timestamp_nanos_opt().unwrap_or(0);
                let ts = Utc::now().timestamp();

                // Check if this looks like an ACP message
                let actual_stream = if is_acp_message(&line) {
                    "acp"
                } else {
                    stream
                };

                let msg = RelayToServer::AgentOutput {
                    agent_id: agent_id.clone(),
                    session_id: session_id.clone(),
                    seq,
                    ts,
                    stream: actual_stream.to_string(),
                    message: line,
                };

                if output_tx.send(msg).await.is_err() {
                    tracing::error!(session_id = %session_id, "output channel closed");
                    break;
                }
            }
            tracing::debug!(session_id = %session_id, stream = %stream, "output reader ended");
        });
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

fn is_acp_message(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('{') {
        return false;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    let Some(ty) = obj.get("type").and_then(|v| v.as_str()) else {
        return false;
    };
    matches!(
        ty,
        "tool_call"
            | "tool_call_update"
            | "agent_message"
            | "agent_thought"
            | "user_message"
            | "permission_request"
            | "run_status"
    )
}
