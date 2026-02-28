use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::RelayConfig;
use crate::session::SessionManager;
use todoki_protocol::{
    RelayToServer, SendInputParams, ServerToRelay, SpawnSessionParams,
};

const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const BUFFER_SIZE: usize = 4096;

pub struct Relay {
    config: RelayConfig,
    relay_id: String,
}

impl Relay {
    pub fn new(config: RelayConfig) -> Self {
        let relay_id = generate_relay_id();
        Self { config, relay_id }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Create a persistent buffer channel
        // All session output goes here first, then forwarded to WebSocket
        let (buffer_tx, buffer_rx) = mpsc::channel::<RelayToServer>(BUFFER_SIZE);

        // Create session manager once - persists across reconnects
        let session_manager = Arc::new(SessionManager::new(
            buffer_tx.clone(),
            self.config.safe_paths().to_vec(),
        ));

        // Start Event Bus stream handler (runs in background)
        let event_stream_url = format!(
            "{}/ws/event-bus?kinds=relay.*,permission.responded&relay_id={}&token={}",
            self.config.server_url().replace("ws://", "ws://").replace("wss://", "wss://"),
            self.relay_id,
            self.config.token
        );

        let _event_handle = tokio::spawn(Self::run_event_stream(
            event_stream_url,
            session_manager.clone(),
            buffer_tx.clone(),
            self.relay_id.clone(),
        ));

        let mut reconnect_delay = RECONNECT_DELAY;

        // Wrap buffer_rx in Option so we can take ownership in the loop
        let mut buffer_rx = Some(buffer_rx);

        loop {
            let rx = buffer_rx.take().expect("buffer_rx should be available");

            match self
                .run_connection(session_manager.clone(), buffer_tx.clone(), rx)
                .await
            {
                ConnectionResult::Reconnect(returned_rx) => {
                    // Put the receiver back for next iteration
                    buffer_rx = Some(returned_rx);

                    tracing::info!(
                        delay_secs = reconnect_delay.as_secs(),
                        "connection lost, reconnecting..."
                    );
                    tokio::time::sleep(reconnect_delay).await;
                    reconnect_delay = std::cmp::min(reconnect_delay * 2, MAX_RECONNECT_DELAY);
                }
                ConnectionResult::ReconnectImmediate(returned_rx) => {
                    // Successfully ran, reset delay and reconnect immediately
                    buffer_rx = Some(returned_rx);
                    reconnect_delay = RECONNECT_DELAY;
                }
                ConnectionResult::FatalError(e) => {
                    tracing::error!(error = %e, "fatal error, stopping relay");
                    session_manager.stop_all().await;
                    return Err(e);
                }
            }
        }
    }

    async fn run_connection(
        &mut self,
        session_manager: Arc<SessionManager>,
        buffer_tx: mpsc::Sender<RelayToServer>,
        mut buffer_rx: mpsc::Receiver<RelayToServer>,
    ) -> ConnectionResult {
        let url = self.config.server_url();
        tracing::info!(url = %url, "connecting to server");

        // Add token to URL
        let connect_url = if url.contains('?') {
            format!("{}&token={}", url, self.config.token)
        } else {
            format!("{}?token={}", url, self.config.token)
        };

        let (ws_stream, _) = match connect_async(&connect_url).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to connect");
                return ConnectionResult::Reconnect(buffer_rx);
            }
        };
        tracing::info!("connected to server");

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Send registration with stable relay ID
        let register_msg = RelayToServer::Register {
            relay_id: self.relay_id.clone(),
            name: self.config.relay_name(),
            role: self.config.role(),
            safe_paths: self.config.safe_paths().to_vec(),
            labels: self.config.labels().clone(),
            projects: self.config.projects().to_vec(),
            setup_script: self.config.setup_script().map(|s| s.to_string()),
        };
        let msg_text = match serde_json::to_string(&register_msg) {
            Ok(t) => t,
            Err(e) => return ConnectionResult::FatalError(e.into()),
        };
        if let Err(e) = ws_write.send(Message::Text(msg_text)).await {
            tracing::error!(error = %e, "failed to send registration");
            return ConnectionResult::Reconnect(buffer_rx);
        }
        tracing::info!(
            relay_id = %self.relay_id,
            name = %self.config.relay_name(),
            "registration sent"
        );

        // Channel to signal shutdown to forwarder
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn forwarder task: buffer_rx -> WebSocket
        let forwarder_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("forwarder received shutdown signal");
                        break;
                    }
                    // Forward messages from buffer to WebSocket
                    msg = buffer_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                let msg_text = match serde_json::to_string(&msg) {
                                    Ok(text) => text,
                                    Err(e) => {
                                        tracing::error!(error = %e, "failed to serialize message");
                                        continue;
                                    }
                                };
                                if ws_write.send(Message::Text(msg_text)).await.is_err() {
                                    tracing::warn!("websocket send failed, stopping forwarder");
                                    break;
                                }
                            }
                            None => {
                                // Buffer channel closed (shouldn't happen normally)
                                tracing::warn!("buffer channel closed");
                                break;
                            }
                        }
                    }
                }
            }
            // Return the receiver so it can be reused
            buffer_rx
        });

        // Process inbound messages from server
        let mut was_registered = false;
        let disconnect_reason = loop {
            let msg = match ws_read.next().await {
                Some(Ok(Message::Text(text))) => text,
                Some(Ok(Message::Ping(_))) => {
                    let _ = buffer_tx.send(RelayToServer::Pong).await;
                    continue;
                }
                Some(Ok(Message::Close(_))) => {
                    tracing::info!("server closed connection");
                    break "server closed";
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    tracing::warn!(error = %e, "websocket error, will reconnect");
                    break "websocket error";
                }
                None => {
                    tracing::info!("websocket stream ended");
                    break "stream ended";
                }
            };

            let server_msg: ServerToRelay = match serde_json::from_str(&msg) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, msg = %msg, "failed to parse server message");
                    continue;
                }
            };

            match server_msg {
                ServerToRelay::Registered { relay_id } => {
                    was_registered = true;
                    tracing::info!(relay_id = %relay_id, "registered with server");
                }

                // RpcRequest is deprecated - commands come via Event Bus now
                ServerToRelay::RpcRequest { id, .. } => {
                    tracing::warn!(request_id = %id, "received deprecated RpcRequest, ignoring");
                }

                // PermissionResponse is deprecated - comes via Event Bus now
                ServerToRelay::PermissionResponse { request_id, .. } => {
                    tracing::warn!(request_id = %request_id, "received deprecated PermissionResponse, ignoring");
                }

                ServerToRelay::Ping => {
                    let _ = buffer_tx.send(RelayToServer::Pong).await;
                }
            }
        };

        tracing::info!(reason = disconnect_reason, "disconnected from server");

        // Signal forwarder to stop
        let _ = shutdown_tx.send(()).await;

        // Wait for forwarder to return the receiver
        let returned_rx = match forwarder_handle.await {
            Ok(rx) => rx,
            Err(e) => {
                tracing::error!(error = %e, "forwarder task panicked");
                // Create a new channel pair - we'll lose buffered messages
                let (new_tx, new_rx) = mpsc::channel::<RelayToServer>(BUFFER_SIZE);
                // Update session manager with new sender
                // This is a fallback - shouldn't normally happen
                drop(new_tx);
                new_rx
            }
        };

        tracing::info!("keeping sessions alive, buffered messages will be sent on reconnect");

        if was_registered {
            ConnectionResult::ReconnectImmediate(returned_rx)
        } else {
            ConnectionResult::Reconnect(returned_rx)
        }
    }

    /// Event Bus stream handler (runs in background)
    ///
    /// Connects to /ws/event-bus and processes command events from server
    async fn run_event_stream(
        url: String,
        session_manager: Arc<SessionManager>,
        buffer_tx: mpsc::Sender<RelayToServer>,
        relay_id: String,
    ) {
        loop {
            tracing::info!(url = %url, "connecting to event bus");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("event bus stream connected");
                    let (mut write, mut read) = ws_stream.split();

                    while let Some(msg_result) = read.next().await {
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                    if let Some(response) = Self::handle_event_message(
                                        ws_msg,
                                        &session_manager,
                                        &relay_id,
                                    )
                                    .await
                                    {
                                        let _ = buffer_tx.send(response).await;
                                    }
                                }
                            }
                            Ok(Message::Ping(_)) => {
                                let _ = write.send(Message::Pong(vec![])).await;
                            }
                            Ok(Message::Close(_)) | Err(_) => {
                                tracing::warn!("event stream closed");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to connect to event bus");
                }
            }

            tracing::info!("reconnecting to event bus in 5s");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Handle Event Bus WebSocket message
    async fn handle_event_message(
        msg: WsMessage,
        session_manager: &SessionManager,
        relay_id: &str,
    ) -> Option<RelayToServer> {
        if msg.msg_type != "event" {
            return None;
        }

        let kind = msg.kind.as_ref()?;
        let data = msg.data.as_ref()?;

        match kind.as_str() {
            "relay.spawn_requested" => {
                let request_id = data.get("request_id")?.as_str()?.to_string();
                let agent_id = data.get("agent_id")?.as_str()?;
                let session_id = data.get("session_id")?.as_str()?;
                let workdir = data.get("workdir")?.as_str()?;
                let command = data.get("command")?.as_str()?;
                let args: Vec<String> =
                    serde_json::from_value(data.get("args")?.clone()).ok()?;
                let env: std::collections::HashMap<String, String> =
                    serde_json::from_value(data.get("env")?.clone()).unwrap_or_default();

                let params = SpawnSessionParams {
                    agent_id: agent_id.to_string(),
                    session_id: session_id.to_string(),
                    workdir: workdir.to_string(),
                    command: command.to_string(),
                    args,
                    env,
                    setup_script: None,
                };

                match session_manager.spawn(params).await {
                    Ok(_result) => {
                        tracing::info!(
                            request_id = %request_id,
                            session_id = %session_id,
                            "spawn succeeded"
                        );
                        Some(RelayToServer::EmitEvent {
                            kind: "relay.spawn_completed".to_string(),
                            data: serde_json::json!({
                                "request_id": request_id,
                                "session_id": session_id,
                                "relay_id": relay_id,
                            }),
                        })
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = %request_id,
                            error = %e,
                            "spawn failed"
                        );
                        Some(RelayToServer::EmitEvent {
                            kind: "relay.spawn_failed".to_string(),
                            data: serde_json::json!({
                                "request_id": request_id,
                                "session_id": session_id,
                                "relay_id": relay_id,
                                "error": e.to_string(),
                            }),
                        })
                    }
                }
            }

            "relay.stop_requested" => {
                let session_id = data.get("session_id")?.as_str()?;
                let _ = session_manager.stop(session_id).await;
                None // Fire-and-forget
            }

            "relay.input_requested" => {
                let session_id = data.get("session_id")?.as_str()?;
                let input = data.get("input")?.as_str()?;

                let params = SendInputParams {
                    session_id: session_id.to_string(),
                    input: input.to_string(),
                };

                let _ = session_manager.send_input(params).await;
                None
            }

            "permission.responded" => {
                let request_id = data.get("request_id")?.as_str()?;
                let session_id = data.get("session_id")?.as_str()?;
                let outcome = Self::parse_permission_outcome(data.get("outcome")?)?;

                let _ = session_manager
                    .respond_permission(session_id, request_id.to_string(), outcome)
                    .await;
                None
            }

            _ => None,
        }
    }

    /// Parse permission outcome from Event Bus data
    fn parse_permission_outcome(
        value: &Value,
    ) -> Option<agent_client_protocol::RequestPermissionOutcome> {
        if let Some(option_id) = value.get("selected").and_then(|v| v.as_str()) {
            Some(
                agent_client_protocol::RequestPermissionOutcome::Selected(
                    agent_client_protocol::SelectedPermissionOutcome::new(option_id.to_string()),
                ),
            )
        } else if value.get("cancelled").is_some() {
            Some(agent_client_protocol::RequestPermissionOutcome::Cancelled)
        } else {
            None
        }
    }
}

/// WebSocket message from Event Bus
#[derive(Debug, serde::Deserialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[allow(dead_code)]
    cursor: Option<i64>,
    kind: Option<String>,
    data: Option<Value>,
}

enum ConnectionResult {
    /// Reconnect after delay, with the buffer receiver
    Reconnect(mpsc::Receiver<RelayToServer>),
    /// Reconnect immediately (was connected successfully), reset backoff
    ReconnectImmediate(mpsc::Receiver<RelayToServer>),
    /// Fatal error, stop relay
    FatalError(anyhow::Error),
}

/// Generate a stable relay ID based on machine ID
fn generate_relay_id() -> String {
    let machine_id = match machine_uid::get() {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "failed to get machine id, using hostname");
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    let hash = hasher.finalize();
    let relay_id = hex::encode(&hash[..16]);

    tracing::info!(relay_id = %relay_id, "generated stable relay ID");
    relay_id
}
