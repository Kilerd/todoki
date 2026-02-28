use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use agent_client_protocol::{
    Agent, CancelNotification, Client, ClientCapabilities, ClientSideConnection, ContentBlock,
    Implementation, InitializeRequest, NewSessionRequest, PromptRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, ToolCall, ToolCallUpdate,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::event_bus_client::EventBusClient;
use crate::relay::RelayOutput;

/// Regex for detecting GitHub PR URLs
static PR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"https://github\.com/([^/]+)/([^/]+)/pull/(\d+)").unwrap());

/// ACP session handle for sending commands
#[derive(Clone)]
pub struct AcpHandle {
    pub acp_session_id: String,
    tx: mpsc::Sender<AcpCommand>,
}

impl AcpHandle {
    /// Send a prompt and return a receiver that signals when this prompt completes.
    pub async fn prompt(&self, input: String) -> anyhow::Result<oneshot::Receiver<()>> {
        let (done_tx, done_rx) = oneshot::channel();
        self.tx
            .send(AcpCommand::Prompt { input, done_tx })
            .await
            .map_err(|_| anyhow::anyhow!("acp channel closed"))?;
        Ok(done_rx)
    }

    pub async fn cancel(&self) -> anyhow::Result<()> {
        self.tx
            .send(AcpCommand::Cancel)
            .await
            .map_err(|_| anyhow::anyhow!("acp channel closed"))
    }

    pub async fn respond_permission(
        &self,
        request_id: String,
        outcome: RequestPermissionOutcome,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            acp_session_id = %self.acp_session_id,
            request_id = %request_id,
            "AcpHandle::respond_permission: sending to command channel"
        );

        self.tx
            .send(AcpCommand::RespondPermission {
                request_id: request_id.clone(),
                outcome,
            })
            .await
            .map_err(|_| anyhow::anyhow!("acp channel closed"))?;

        tracing::debug!(
            acp_session_id = %self.acp_session_id,
            request_id = %request_id,
            "AcpHandle::respond_permission: sent successfully"
        );

        Ok(())
    }
}

#[derive(Debug)]
enum AcpCommand {
    Prompt {
        input: String,
        /// Sender to signal when this specific prompt completes
        done_tx: oneshot::Sender<()>,
    },
    Cancel,
    RespondPermission {
        request_id: String,
        outcome: RequestPermissionOutcome,
    },
}

/// Buffer state for aggregating output
struct OutputBufferState {
    current_stream: Option<String>,
    messages: Vec<String>,
}

/// Event sink for ACP output
#[derive(Clone)]
struct AcpEventSink {
    output_tx: mpsc::Sender<RelayOutput>,
    agent_id: String,
    session_id: String,
    seq_counter: Arc<AtomicI64>,
    /// Buffer for aggregating output (flush on stream type change)
    buffer: Arc<Mutex<OutputBufferState>>,
    /// Client for emitting events to event-bus
    event_bus: EventBusClient,
}

impl AcpEventSink {
    fn new(
        output_tx: mpsc::Sender<RelayOutput>,
        agent_id: String,
        session_id: String,
        event_bus: EventBusClient,
    ) -> Self {
        // Initialize seq with current timestamp to maintain global ordering across sessions
        let initial_seq = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        Self {
            output_tx,
            agent_id,
            session_id,
            seq_counter: Arc::new(AtomicI64::new(initial_seq)),
            buffer: Arc::new(Mutex::new(OutputBufferState {
                current_stream: None,
                messages: Vec::new(),
            })),
            event_bus,
        }
    }

    async fn emit_system(&self, message: String) {
        self.emit_raw("system", message).await;
    }

    async fn emit_raw(&self, stream: &str, message: String) {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0);

        tracing::debug!(
            session_id = %self.session_id,
            stream = %stream,
            seq = seq,
            message_len = message.len(),
            "emitting agent output"
        );

        // Check if stream type changed - if so, flush previous buffer first
        {
            let mut buffer = self.buffer.lock().await;
            if let Some(ref current) = buffer.current_stream {
                if current != stream && !buffer.messages.is_empty() {
                    // Stream type changed, flush previous
                    self.flush_buffer_inner(current, &buffer.messages, ts).await;
                    buffer.messages.clear();
                }
            }
            buffer.current_stream = Some(stream.to_string());
            buffer.messages.push(message.clone());
        }

        // Emit via Event Bus (streaming, for real-time display)
        let msg = RelayOutput::EmitEvent {
            kind: "relay.agent_output".to_string(),
            data: serde_json::json!({
                "agent_id": self.agent_id,
                "session_id": self.session_id,
                "seq": seq,
                "ts": ts,
                "stream": stream,
                "message": message,
            }),
        };

        let _ = self.output_tx.send(msg).await;
    }

    /// Internal: emit batch event to event-bus
    async fn flush_buffer_inner(&self, stream: &str, messages: &[String], ts: i64) {
        if messages.is_empty() {
            return;
        }

        let data = serde_json::json!({
            "agent_id": self.agent_id,
            "session_id": self.session_id,
            "ts": ts,
            "stream": stream,
            "messages": messages,
        });

        self.event_bus
            .emit_fire_and_forget("agent.output_batch", data)
            .await;
    }

    /// Flush remaining buffer (call on prompt completion)
    async fn flush_buffer(&self) {
        let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let (stream, messages) = {
            let mut buffer = self.buffer.lock().await;
            let stream = buffer.current_stream.take();
            let messages = std::mem::take(&mut buffer.messages);
            (stream, messages)
        };

        if let Some(stream) = stream {
            self.flush_buffer_inner(&stream, &messages, ts).await;
        }
    }

    async fn emit_update(&self, update: SessionUpdate) {
        tracing::debug!(
            session_id = %self.session_id,
            update_type = %std::any::type_name_of_val(&update),
            "processing session update"
        );

        // Check for artifacts in tool call updates
        if let SessionUpdate::ToolCallUpdate(ref tool_update) = update {
            self.detect_artifacts(tool_update).await;
        }

        // Determine stream type based on SessionUpdate variant
        let stream = match &update {
            SessionUpdate::AgentMessageChunk(_) => "assistant",
            SessionUpdate::AgentThoughtChunk(_) => "thinking",
            SessionUpdate::ToolCall(_) => "tool_use",
            SessionUpdate::ToolCallUpdate(_) => "tool_result",
            SessionUpdate::UserMessageChunk(_) => "user",
            SessionUpdate::Plan(_) => "plan",
            _ => "system",
        };

        if let Some(value) = update_to_event(update) {
            let message = value.to_string();
            self.emit_raw(stream, message).await;
        }
    }

    /// Detect and emit artifacts from tool call output (e.g., GitHub PR URLs)
    async fn detect_artifacts(&self, update: &ToolCallUpdate) {
        if let Some(raw_output) = &update.fields.raw_output {
            if let Some(output_str) = raw_output.as_str() {
                // Detect GitHub PR URLs
                for caps in PR_REGEX.captures_iter(output_str) {
                    let url = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
                    let owner = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                    let repo = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
                    let number: u32 = caps
                        .get(3)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(0);

                    tracing::info!(
                        session_id = %self.session_id,
                        url = %url,
                        owner = %owner,
                        repo = %repo,
                        number = number,
                        "detected GitHub PR artifact"
                    );

                    let artifact_data = serde_json::json!({
                        "session_id": self.session_id,
                        "agent_id": self.agent_id,
                        "artifact_type": "github_pr",
                        "data": {
                            "url": url,
                            "owner": owner,
                            "repo": repo,
                            "number": number,
                        },
                    });

                    // Send to server via WebSocket (for artifacts table)
                    let msg = RelayOutput::EmitEvent {
                        kind: "relay.artifact".to_string(),
                        data: artifact_data.clone(),
                    };
                    let _ = self.output_tx.send(msg).await;

                    // Also emit to event-bus via HTTP for persistence/replay
                    self.event_bus
                        .emit_fire_and_forget("artifact.created", artifact_data)
                        .await;
                }
            }
        }
    }
}

/// Pending permission request state
struct PendingPermission {
    request_id: String,
    response_tx: oneshot::Sender<RequestPermissionOutcome>,
}

/// Permission manager for a single pending request.
/// In single-session mode, only one permission request can be pending at a time.
struct PermissionManager {
    pending: Mutex<Option<PendingPermission>>,
    output_tx: mpsc::Sender<RelayOutput>,
    agent_id: String,
    session_id: String,
    event_bus: EventBusClient,
}

impl PermissionManager {
    fn new(
        output_tx: mpsc::Sender<RelayOutput>,
        agent_id: String,
        session_id: String,
        event_bus: EventBusClient,
    ) -> Self {
        Self {
            pending: Mutex::new(None),
            output_tx,
            agent_id,
            session_id,
            event_bus,
        }
    }

    async fn create_request(
        &self,
        args: &RequestPermissionRequest,
    ) -> anyhow::Result<(String, oneshot::Receiver<RequestPermissionOutcome>)> {
        let request_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            session_id = %self.session_id,
            request_id = %request_id,
            tool_call_id = %args.tool_call.tool_call_id,
            "PermissionManager::create_request: creating permission request"
        );

        // IMPORTANT: Register in pending BEFORE sending to server to avoid race condition
        // where response arrives before we're ready to receive it
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            // Replace any existing pending request (shouldn't happen in normal flow)
            if pending.is_some() {
                tracing::warn!(
                    session_id = %self.session_id,
                    "replacing existing pending permission request"
                );
            }
            *pending = Some(PendingPermission {
                request_id: request_id.clone(),
                response_tx: tx,
            });
            tracing::debug!(
                session_id = %self.session_id,
                request_id = %request_id,
                "PermissionManager::create_request: request registered"
            );
        }

        // Send permission request to server via WebSocket (for real-time handling)
        let event_data = serde_json::json!({
            "request_id": request_id,
            "agent_id": self.agent_id,
            "session_id": self.session_id,
            "tool_call_id": args.tool_call.tool_call_id.to_string(),
            "options": serde_json::to_value(&args.options).unwrap_or_default(),
            "tool_call": serde_json::to_value(&args.tool_call).unwrap_or_default(),
        });

        let msg = RelayOutput::EmitEvent {
            kind: "relay.permission_request".to_string(),
            data: event_data.clone(),
        };

        if let Err(e) = self.output_tx.send(msg).await {
            // If send fails, clear pending and return error
            let mut pending = self.pending.lock().await;
            *pending = None;
            return Err(anyhow::anyhow!("failed to send permission request: {}", e));
        }

        // Also emit to event-bus via HTTP for persistence/replay
        self.event_bus
            .emit_fire_and_forget("permission.requested", event_data)
            .await;

        tracing::debug!(
            session_id = %self.session_id,
            request_id = %request_id,
            "PermissionManager::create_request: sent to server"
        );

        Ok((request_id, rx))
    }

    async fn respond(&self, request_id: &str, outcome: RequestPermissionOutcome) {
        tracing::debug!(
            session_id = %self.session_id,
            request_id = %request_id,
            "PermissionManager::respond: acquiring pending lock"
        );

        let mut pending = self.pending.lock().await;

        tracing::debug!(
            session_id = %self.session_id,
            request_id = %request_id,
            has_pending = pending.is_some(),
            "PermissionManager::respond: lock acquired"
        );

        if let Some(perm) = pending.take() {
            if perm.request_id == request_id {
                tracing::debug!(
                    session_id = %self.session_id,
                    request_id = %request_id,
                    "PermissionManager::respond: found matching request, sending response"
                );
                let _ = perm.response_tx.send(outcome);
                tracing::debug!(
                    session_id = %self.session_id,
                    request_id = %request_id,
                    "PermissionManager::respond: response sent"
                );
            } else {
                tracing::warn!(
                    session_id = %self.session_id,
                    expected_id = %perm.request_id,
                    actual_id = %request_id,
                    "PermissionManager::respond: request_id mismatch, ignoring response"
                );
                // Put it back since it wasn't for us
                *pending = Some(perm);
            }
        } else {
            tracing::warn!(
                session_id = %self.session_id,
                request_id = %request_id,
                "PermissionManager::respond: no pending request found"
            );
        }
    }
}

/// ACP client implementation
struct AcpClient {
    sink: AcpEventSink,
    permissions: Arc<PermissionManager>,
}

impl AcpClient {
    fn new(sink: AcpEventSink, permissions: Arc<PermissionManager>) -> Self {
        Self { sink, permissions }
    }
}

#[async_trait::async_trait(?Send)]
impl Client for AcpClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, agent_client_protocol::Error> {
        let (request_id, response_rx) = self
            .permissions
            .create_request(&args)
            .await
            .map_err(|e| agent_client_protocol::Error::internal_error().data(e.to_string()))?;

        // Wait for response with timeout (5 minutes)
        let outcome = match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            response_rx,
        )
        .await
        {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => {
                // Channel closed
                pick_allow_option(&args)
            }
            Err(_) => {
                // Timeout - auto-select allow option
                self.sink
                    .emit_system(format!("permission request {} timed out", request_id))
                    .await;
                pick_allow_option(&args)
            }
        };

        Ok(RequestPermissionResponse::new(outcome))
    }

    async fn session_notification(
        &self,
        args: SessionNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        self.sink.emit_update(args.update).await;
        Ok(())
    }
}

fn pick_allow_option(args: &RequestPermissionRequest) -> RequestPermissionOutcome {
    let option_id = args
        .options
        .iter()
        .find(|opt| {
            matches!(
                opt.kind,
                agent_client_protocol::PermissionOptionKind::AllowAlways
                    | agent_client_protocol::PermissionOptionKind::AllowOnce
            )
        })
        .or_else(|| args.options.first())
        .map(|opt| opt.option_id.clone());

    match option_id {
        Some(id) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
        None => RequestPermissionOutcome::Cancelled,
    }
}

/// Spawn an ACP session
pub async fn spawn_acp_session(
    output_tx: mpsc::Sender<RelayOutput>,
    agent_id: String,
    session_id: String,
    workdir: String,
    stdout: ChildStdout,
    stdin: ChildStdin,
    server_url: String,
    token: String,
) -> anyhow::Result<AcpHandle> {
    tracing::debug!(
        session_id = %session_id,
        agent_id = %agent_id,
        workdir = %workdir,
        "spawn_acp_session called"
    );

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AcpCommand>(64);
    let (ready_tx, ready_rx) = oneshot::channel::<Result<String, String>>();

    let event_bus = EventBusClient::new(&server_url, &token);

    let sink = AcpEventSink::new(
        output_tx.clone(),
        agent_id.clone(),
        session_id.clone(),
        event_bus.clone(),
    );
    let permissions = Arc::new(PermissionManager::new(
        output_tx.clone(),
        agent_id.clone(),
        session_id.clone(),
        event_bus,
    ));
    let permissions_for_cmd = permissions.clone();

    // Clone for prompt completed notification
    let prompt_completed_tx = output_tx.clone();
    let prompt_completed_session_id = session_id.clone();

    let session_id_for_thread = session_id.clone();
    std::thread::spawn(move || {
        tracing::debug!(session_id = %session_id_for_thread, "ACP thread started");
        tracing::debug!("building tokio runtime for ACP");
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(error = %e, "failed to build tokio runtime");
                let _ = ready_tx.send(Err(format!("runtime init failed: {}", e)));
                return;
            }
        };
        tracing::debug!("tokio runtime created");

        let local = tokio::task::LocalSet::new();
        runtime.block_on(local.run_until(async move {
            tracing::debug!("inside LocalSet, creating ACP client");
            let client = AcpClient::new(sink.clone(), permissions);
            let outgoing = stdin.compat_write();
            let incoming = stdout.compat();

            tracing::debug!("creating ClientSideConnection");
            let (conn, io_task) = ClientSideConnection::new(client, outgoing, incoming, |fut| {
                tokio::task::spawn_local(fut);
            });
            tracing::debug!("ClientSideConnection created");

            let io_sink = sink.clone();
            tokio::task::spawn_local(async move {
                tracing::debug!("io_task started");
                if let Err(e) = io_task.await {
                    tracing::error!(error = %e, "ACP io error");
                    io_sink.emit_system(format!("acp io error: {}", e)).await;
                }
                tracing::debug!("io_task ended");
            });

            // Initialize protocol
            tracing::debug!("sending ACP initialize request");
            let init = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(ClientCapabilities::default())
                .client_info(Implementation::new("todoki-relay", env!("CARGO_PKG_VERSION")));

            if let Err(e) = conn.initialize(init).await {
                tracing::error!(error = %e, "ACP initialize failed");
                let _ = ready_tx.send(Err(format!("acp init failed: {}", e)));
                return;
            }
            tracing::debug!("ACP initialize succeeded");

            // Create new session
            tracing::debug!(workdir = %workdir, "creating new ACP session");
            let cwd = PathBuf::from(&workdir);
            let acp_session = match conn.new_session(NewSessionRequest::new(cwd)).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "ACP new_session failed");
                    let _ = ready_tx.send(Err(format!("new_session failed: {}", e)));
                    return;
                }
            };

            let acp_session_id = acp_session.session_id.to_string();
            tracing::info!(acp_session_id = %acp_session_id, "ACP session created");
            let _ = ready_tx.send(Ok(acp_session_id.clone()));

            // Wrap conn in Rc so it can be shared with spawn_local tasks
            let conn = Rc::new(conn);

            // Command loop
            tracing::debug!(acp_session_id = %acp_session_id, "entering ACP command loop");
            while let Some(cmd) = cmd_rx.recv().await {
                tracing::debug!(acp_session_id = %acp_session_id, cmd = ?cmd, "received ACP command");
                match cmd {
                    AcpCommand::Prompt { input: ref prompt, done_tx: prompt_done_tx } => {
                        tracing::info!(
                            acp_session_id = %acp_session_id,
                            prompt_len = prompt.len(),
                            "sending prompt to agent"
                        );
                        tracing::debug!(
                            acp_session_id = %acp_session_id,
                            prompt = %prompt,
                            "prompt content"
                        );

                        // Spawn prompt execution to avoid blocking the command loop
                        // This allows RespondPermission commands to be processed while
                        // the prompt is waiting for permission responses
                        let conn = conn.clone();
                        let acp_session_id = acp_session_id.clone();
                        let sink = sink.clone();
                        let prompt = prompt.clone();
                        let prompt_completed_tx = prompt_completed_tx.clone();
                        let prompt_completed_session_id = prompt_completed_session_id.clone();
                        tokio::task::spawn_local(async move {
                            let request = PromptRequest::new(
                                acp_session_id.clone(),
                                vec![ContentBlock::Text(
                                    agent_client_protocol::TextContent::new(prompt),
                                )],
                            );
                            let result = conn.prompt(request).await;
                            let success = result.is_ok();
                            let error = result.as_ref().err().map(|e| e.to_string());

                            if let Err(ref e) = result {
                                tracing::error!(
                                    acp_session_id = %acp_session_id,
                                    error = %e,
                                    "prompt request failed"
                                );
                                sink.emit_system(format!("prompt error: {}", e)).await;
                            } else {
                                tracing::debug!(acp_session_id = %acp_session_id, "prompt request completed");
                            }

                            // Flush remaining output buffer to event-bus
                            sink.flush_buffer().await;

                            // Send prompt completed notification via Event Bus
                            let msg = RelayOutput::EmitEvent {
                                kind: "relay.prompt_completed".to_string(),
                                data: serde_json::json!({
                                    "session_id": prompt_completed_session_id,
                                    "success": success,
                                    "error": error,
                                }),
                            };
                            let _ = prompt_completed_tx.send(msg).await;

                            // Signal this specific prompt is done
                            let _ = prompt_done_tx.send(());
                        });
                    }
                    AcpCommand::Cancel => {
                        tracing::info!(acp_session_id = %acp_session_id, "cancelling current operation");
                        let request = CancelNotification::new(acp_session_id.clone());
                        if let Err(e) = conn.cancel(request).await {
                            tracing::error!(
                                acp_session_id = %acp_session_id,
                                error = %e,
                                "cancel request failed"
                            );
                            sink.emit_system(format!("cancel error: {}", e)).await;
                        }
                    }
                    AcpCommand::RespondPermission { request_id, outcome } => {
                        tracing::info!(
                            acp_session_id = %acp_session_id,
                            request_id = %request_id,
                            outcome = ?outcome,
                            "responding to permission request"
                        );
                        permissions_for_cmd.respond(&request_id, outcome).await;
                    }
                }
            }
            tracing::debug!(acp_session_id = %acp_session_id, "ACP command loop ended");
        }));
    });

    tracing::debug!(session_id = %session_id, "waiting for ACP ready signal");
    match ready_rx.await {
        Ok(Ok(acp_session_id)) => {
            tracing::info!(
                session_id = %session_id,
                acp_session_id = %acp_session_id,
                "ACP session ready"
            );
            Ok(AcpHandle {
                acp_session_id,
                tx: cmd_tx,
            })
        }
        Ok(Err(e)) => {
            tracing::error!(session_id = %session_id, error = %e, "ACP initialization error");
            Err(anyhow::anyhow!(e))
        }
        Err(_) => {
            tracing::error!(session_id = %session_id, "ACP ready channel dropped");
            Err(anyhow::anyhow!("acp session init cancelled"))
        }
    }
}

// Helper functions for converting updates to events

fn update_to_event(update: SessionUpdate) -> Option<Value> {
    match &update {
        SessionUpdate::UserMessageChunk(chunk) => {
            Some(json_message("user_message", &chunk.content))
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            Some(json_message("agent_message", &chunk.content))
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            Some(json_message("agent_thought", &chunk.content))
        }
        SessionUpdate::ToolCall(tool_call) => Some(json_tool_call(tool_call)),
        SessionUpdate::ToolCallUpdate(update) => Some(json_tool_call_update(update)),
        SessionUpdate::Plan(plan) => Some(serde_json::json!({
            "type": "plan",
            "plan": plan,
        })),
        SessionUpdate::AvailableCommandsUpdate(update) => Some(serde_json::json!({
            "type": "available_commands",
            "commands": update.available_commands,
            "meta": update.meta,
        })),
        SessionUpdate::CurrentModeUpdate(update) => Some(serde_json::json!({
            "type": "current_mode",
            "current_mode_id": update.current_mode_id,
            "meta": update.meta,
        })),
        _ => serde_json::to_value(&update)
            .ok()
            .map(|payload| serde_json::json!({ "type": "session_update", "payload": payload })),
    }
}

fn json_message(kind: &str, content: &ContentBlock) -> Value {
    let text = match content {
        ContentBlock::Text(t) => t.text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    serde_json::json!({
        "type": kind,
        "text": text,
        "chunk": true
    })
}

fn json_tool_call(tool_call: &ToolCall) -> Value {
    serde_json::json!({
        "type": "tool_call",
        "id": tool_call.tool_call_id.to_string(),
        "title": tool_call.title,
        "kind": serde_json::to_value(&tool_call.kind).unwrap_or(Value::Null),
        "status": serde_json::to_value(&tool_call.status).unwrap_or(Value::Null),
        "content": serde_json::to_value(&tool_call.content).unwrap_or(Value::Null),
        "raw_input": tool_call.raw_input,
        "raw_output": tool_call.raw_output,
        "meta": tool_call.meta
    })
}

fn json_tool_call_update(update: &ToolCallUpdate) -> Value {
    let mut obj = Map::new();
    obj.insert("type".to_string(), Value::String("tool_call_update".to_string()));
    obj.insert("id".to_string(), Value::String(update.tool_call_id.to_string()));

    let fields = &update.fields;
    if let Some(kind) = &fields.kind {
        obj.insert("kind".to_string(), serde_json::to_value(kind).unwrap_or(Value::Null));
    }
    if let Some(status) = &fields.status {
        obj.insert("status".to_string(), serde_json::to_value(status).unwrap_or(Value::Null));
    }
    if let Some(title) = &fields.title {
        obj.insert("title".to_string(), Value::String(title.clone()));
    }
    if let Some(content) = &fields.content {
        obj.insert("content".to_string(), serde_json::to_value(content).unwrap_or(Value::Null));
    }
    if let Some(raw_input) = &fields.raw_input {
        obj.insert("raw_input".to_string(), raw_input.clone());
    }
    if let Some(raw_output) = &fields.raw_output {
        obj.insert("raw_output".to_string(), raw_output.clone());
    }
    if let Some(meta) = &update.meta {
        obj.insert("meta".to_string(), serde_json::to_value(meta).unwrap_or(Value::Null));
    }

    Value::Object(obj)
}
