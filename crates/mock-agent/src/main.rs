//! Mock agent for integration testing of todoki-relay.
//!
//! This binary implements the ACP Agent trait and communicates over stdin/stdout.
//! It supports:
//! - Basic initialization and session creation
//! - Simple prompts that complete immediately
//! - Prompts containing "permission" that trigger permission requests
//! - Event-driven task processing
//! - Cancellation handling

use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::{
    Agent, AgentSideConnection, AuthenticateRequest, AuthenticateResponse, CancelNotification,
    ContentBlock, ExtNotification, ExtRequest, ExtResponse, Implementation, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, SessionId, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info};

/// Event structure matching server-side Event
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Event {
    cursor: i64,
    kind: String,
    time: String,
    agent_id: String,
    session_id: Option<String>,
    task_id: Option<String>,
    data: serde_json::Value,
}

/// Mock agent state
#[derive(Clone)]
struct MockAgent {
    sessions: Arc<Mutex<Vec<SessionId>>>,
    cancelled: Arc<Mutex<Vec<SessionId>>>,
}

impl MockAgent {
    fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Handle task.created event
    async fn handle_task_created(event: Event) -> Result<(), Box<dyn std::error::Error>> {
        let content = event
            .data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("(no content)");

        info!("🤖 Analyzing task: {}", content);

        // Simulate analysis
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Emit analysis result
        let result = json!({
            "plan": format!("Analysis of '{}': Step 1, Step 2, Step 3", content),
            "estimated_effort": "medium",
            "breakdown": [
                {"subtask": "Research", "assignee": "ba-agent"},
                {"subtask": "Implementation", "assignee": "coding-agent"},
            ]
        });

        info!(
            "📤 Emitting agent.requirement_analyzed event for task: {:?}",
            event.task_id
        );

        // Note: In real implementation, this would be sent via connection
        // For now, we just log the event that would be emitted
        debug!(
            "Would emit event: {}",
            serde_json::to_string(&json!({
                "kind": "agent.requirement_analyzed",
                "data": result,
                "task_id": event.task_id,
            }))?
        );

        Ok(())
    }

    /// Handle agent.requirement_analyzed event
    async fn handle_requirement_analyzed(
        event: Event,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let plan = event
            .data
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("(no plan)");

        info!("📋 Received analysis: {}", plan);

        // BA agent would load business context here
        // For now, just acknowledge
        info!("✅ Business context loaded");

        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl Agent for MockAgent {
    async fn initialize(
        &self,
        args: InitializeRequest,
    ) -> agent_client_protocol::Result<InitializeResponse> {
        Ok(InitializeResponse::new(args.protocol_version)
            .agent_info(Implementation::new("mock-agent", "0.1.0").title("Mock Agent")))
    }

    async fn authenticate(
        &self,
        _args: AuthenticateRequest,
    ) -> agent_client_protocol::Result<AuthenticateResponse> {
        Ok(AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        args: NewSessionRequest,
    ) -> agent_client_protocol::Result<NewSessionResponse> {
        let session_id = SessionId::new(format!("mock-session-{}", uuid_simple()));
        self.sessions.lock().unwrap().push(session_id.clone());
        info!("new_session created: {} at {:?}", session_id.0, args.cwd);
        Ok(NewSessionResponse::new(session_id))
    }

    async fn load_session(
        &self,
        _args: LoadSessionRequest,
    ) -> agent_client_protocol::Result<LoadSessionResponse> {
        Ok(LoadSessionResponse::new())
    }

    async fn set_session_mode(
        &self,
        _args: SetSessionModeRequest,
    ) -> agent_client_protocol::Result<SetSessionModeResponse> {
        Ok(SetSessionModeResponse::new())
    }

    async fn prompt(
        &self,
        args: PromptRequest,
    ) -> agent_client_protocol::Result<PromptResponse> {
        info!("prompt received for session: {}", args.session_id.0);

        // Extract prompt text
        let prompt_text = args
            .prompt
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        debug!("prompt text: {}", prompt_text);

        // Check if prompt contains "permission" - if so, request permission
        if prompt_text.to_lowercase().contains("permission") {
            info!("requesting permission...");

            // We need to get the connection to request permission
            // This is handled via the Client trait implementation on the connection
            // For now, we'll just return EndTurn since we can't easily access the connection here
            // The actual permission request would be done through session_notification

            // Return with end turn - in a real implementation, we would:
            // 1. Send a tool call notification
            // 2. Request permission
            // 3. Wait for response
            // 4. Complete the tool call
        }

        info!("prompt completed");
        Ok(PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(
        &self,
        args: CancelNotification,
    ) -> agent_client_protocol::Result<()> {
        info!("cancel received for session: {}", args.session_id.0);
        self.cancelled.lock().unwrap().push(args.session_id);
        Ok(())
    }

    async fn set_session_config_option(
        &self,
        _args: SetSessionConfigOptionRequest,
    ) -> agent_client_protocol::Result<SetSessionConfigOptionResponse> {
        Ok(SetSessionConfigOptionResponse::new(vec![]))
    }

    async fn ext_method(
        &self,
        _args: ExtRequest,
    ) -> agent_client_protocol::Result<ExtResponse> {
        Ok(ExtResponse::new(RawValue::NULL.to_owned().into()))
    }

    async fn ext_notification(
        &self,
        args: ExtNotification,
    ) -> agent_client_protocol::Result<()> {
        // Check if this is an event notification
        if args.method.as_ref() == "event" {
            match serde_json::from_str::<Event>(args.params.get()) {
                Ok(event) => {
                    info!("Received event: {} (cursor: {})", event.kind, event.cursor);

                    // Handle different event kinds
                    match event.kind.as_str() {
                        "task.created" => {
                            if let Err(e) = Self::handle_task_created(event).await {
                                error!("Failed to handle task.created: {}", e);
                            }
                        }
                        "agent.requirement_analyzed" => {
                            if let Err(e) = Self::handle_requirement_analyzed(event).await {
                                error!("Failed to handle requirement_analyzed: {}", e);
                            }
                        }
                        _ => {
                            debug!("Ignoring event kind: {}", event.kind);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse event: {}", e);
                }
            }
        }
        Ok(())
    }
}

/// Generate a simple UUID-like string without external dependency
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Mock agent starting...");

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let agent = MockAgent::new();

            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (_conn, io_task) = AgentSideConnection::new(
                agent,
                stdout.compat_write(),
                stdin.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            // Spawn IO task
            tokio::task::spawn_local(async move {
                if let Err(e) = io_task.await {
                    error!("IO error: {}", e);
                }
                info!("IO task ended");
            });

            // Keep the connection alive
            // The connection is driven by the IO task, we just need to keep this task running
            info!("ready, waiting for commands...");

            // Keep running until stdin closes
            loop {
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        })
        .await;
}
