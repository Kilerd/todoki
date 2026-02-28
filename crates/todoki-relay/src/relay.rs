use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::RelayConfig;
use crate::session::SessionManager;
use todoki_protocol::{EventPermissionOutcome, SendInputParams};

const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const BUFFER_SIZE: usize = 4096;

/// Client → Server message format for Event Bus WebSocket
#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// Emit an event to the Event Bus
    EmitEvent { kind: String, data: Value },
    /// Pong response to ping
    Pong,
}

/// Server → Client message format from Event Bus WebSocket
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    /// Event notification
    Event {
        #[allow(dead_code)]
        cursor: i64,
        kind: String,
        #[allow(dead_code)]
        time: String,
        #[allow(dead_code)]
        agent_id: String,
        #[allow(dead_code)]
        session_id: Option<String>,
        #[allow(dead_code)]
        task_id: Option<String>,
        data: Value,
    },
    /// Subscription acknowledged
    Subscribed {
        #[allow(dead_code)]
        kinds: Option<Vec<String>>,
        #[allow(dead_code)]
        cursor: i64,
    },
    /// Relay registered confirmation
    Registered {
        relay_id: String,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Heartbeat ping
    Ping,
    /// Heartbeat pong
    #[allow(dead_code)]
    Pong,
    /// Replay complete (ignored)
    ReplayComplete {
        #[allow(dead_code)]
        cursor: i64,
        #[allow(dead_code)]
        count: usize,
    },
}

/// Internal message type for relay output buffer
#[derive(Debug, Clone)]
pub enum RelayOutput {
    /// Emit event to server
    EmitEvent { kind: String, data: Value },
}

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
        let (buffer_tx, buffer_rx) = mpsc::channel::<RelayOutput>(BUFFER_SIZE);

        // Create session manager once - persists across reconnects
        let session_manager = Arc::new(SessionManager::new(
            buffer_tx.clone(),
            self.config.safe_paths().to_vec(),
            self.config.server_url().to_string(),
            self.config.token.clone(),
        ));

        let mut reconnect_delay = RECONNECT_DELAY;

        // Wrap buffer_rx in Option so we can take ownership in the loop
        let mut buffer_rx = Some(buffer_rx);

        loop {
            let rx = buffer_rx.take().expect("buffer_rx should be available");

            match self
                .run_event_bus_connection(session_manager.clone(), buffer_tx.clone(), rx)
                .await
            {
                ConnectionResult::Reconnect(returned_rx) => {
                    buffer_rx = Some(returned_rx);

                    tracing::info!(
                        delay_secs = reconnect_delay.as_secs(),
                        "connection lost, reconnecting..."
                    );
                    tokio::time::sleep(reconnect_delay).await;
                    reconnect_delay = std::cmp::min(reconnect_delay * 2, MAX_RECONNECT_DELAY);
                }
                ConnectionResult::ReconnectImmediate(returned_rx) => {
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

    /// Run Event Bus connection - unified communication channel
    async fn run_event_bus_connection(
        &mut self,
        session_manager: Arc<SessionManager>,
        buffer_tx: mpsc::Sender<RelayOutput>,
        mut buffer_rx: mpsc::Receiver<RelayOutput>,
    ) -> ConnectionResult {
        // Build Event Bus WebSocket URL with relay_id
        let base_url = self.config.server_url();
        let event_bus_url = base_url
            .replace("/ws/relay", "/ws/event-bus")
            .replace("/ws/relays", "/ws/event-bus");

        let url = format!(
            "{}?relay_id={}&kinds=relay.*,permission.responded&token={}",
            if event_bus_url.contains("/ws/event-bus") {
                event_bus_url
            } else {
                format!("{}/ws/event-bus", base_url.trim_end_matches('/'))
            },
            self.relay_id,
            self.config.token
        );

        tracing::info!(url = %url, relay_id = %self.relay_id, "connecting to event bus");

        let (ws_stream, _) = match connect_async(&url).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to connect to event bus");
                return ConnectionResult::Reconnect(buffer_rx);
            }
        };
        tracing::info!("connected to event bus");

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Wait for subscribed acknowledgment first
        let mut subscribed = false;
        while !subscribed {
            match ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                        if matches!(msg, ServerMessage::Subscribed { .. }) {
                            subscribed = true;
                            tracing::debug!("received subscription acknowledgment");
                        }
                    }
                }
                Some(Err(e)) => {
                    tracing::error!(error = %e, "error waiting for subscription ack");
                    return ConnectionResult::Reconnect(buffer_rx);
                }
                None => {
                    tracing::error!("connection closed before subscription ack");
                    return ConnectionResult::Reconnect(buffer_rx);
                }
                _ => {}
            }
        }

        // Send relay.up registration event
        let registration_data = serde_json::json!({
            "relay_id": self.relay_id,
            "name": self.config.relay_name(),
            "role": self.config.role().as_str(),
            "safe_paths": self.config.safe_paths(),
            "labels": self.config.labels(),
            "projects": self.config.projects(),
            "setup_script": self.config.setup_script(),
        });

        let register_msg = ClientMessage::EmitEvent {
            kind: "relay.up".to_string(),
            data: registration_data,
        };

        let msg_text = match serde_json::to_string(&register_msg) {
            Ok(t) => t,
            Err(e) => return ConnectionResult::FatalError(e.into()),
        };

        if let Err(e) = ws_write.send(Message::Text(msg_text)).await {
            tracing::error!(error = %e, "failed to send relay.up");
            return ConnectionResult::Reconnect(buffer_rx);
        }
        tracing::info!(relay_id = %self.relay_id, "sent relay.up registration");

        // Wait for registered confirmation
        let mut registered = false;
        let timeout = tokio::time::timeout(Duration::from_secs(30), async {
            while !registered {
                match ws_read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                            match msg {
                                ServerMessage::Registered { relay_id } => {
                                    tracing::info!(relay_id = %relay_id, "registered with server");
                                    registered = true;
                                }
                                ServerMessage::Error { message } => {
                                    tracing::error!(error = %message, "registration error");
                                    return Err(anyhow::anyhow!("registration error: {}", message));
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Err(anyhow::anyhow!("websocket error: {}", e));
                    }
                    None => {
                        return Err(anyhow::anyhow!("connection closed"));
                    }
                    _ => {}
                }
            }
            Ok(())
        });

        match timeout.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::error!(error = %e, "registration failed");
                return ConnectionResult::Reconnect(buffer_rx);
            }
            Err(_) => {
                tracing::error!("registration timeout");
                return ConnectionResult::Reconnect(buffer_rx);
            }
        }

        // Channel to signal shutdown to forwarder
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn forwarder task: buffer_rx -> WebSocket (via Event Bus emit)
        let forwarder_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("forwarder received shutdown signal");
                        break;
                    }
                    msg = buffer_rx.recv() => {
                        match msg {
                            Some(RelayOutput::EmitEvent { kind, data }) => {
                                let client_msg = ClientMessage::EmitEvent { kind, data };
                                let msg_text = match serde_json::to_string(&client_msg) {
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
                                tracing::warn!("buffer channel closed");
                                break;
                            }
                        }
                    }
                }
            }
            buffer_rx
        });

        // Process inbound messages from server (events)
        let disconnect_reason = loop {
            match ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                        match msg {
                            ServerMessage::Event { kind, data, .. } => {
                                if let Some(response) = Self::handle_server_event(
                                    &kind,
                                    &data,
                                    &session_manager,
                                    &self.relay_id,
                                    &buffer_tx,
                                    self.config.setup_script(),
                                )
                                .await
                                {
                                    let _ = buffer_tx.send(response).await;
                                }
                            }
                            ServerMessage::Ping => {
                                // Server sends JSON-level ping for keep-alive
                                tracing::debug!("received ping from server");
                            }
                            ServerMessage::Error { message } => {
                                tracing::warn!(error = %message, "received error from server");
                            }
                            _ => {}
                        }
                    }
                }
                Some(Ok(Message::Ping(_))) => {
                    // WebSocket level ping - handle via buffer for pong
                    tracing::debug!("received websocket ping");
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
                let (_, new_rx) = mpsc::channel::<RelayOutput>(BUFFER_SIZE);
                new_rx
            }
        };

        tracing::info!("keeping sessions alive, buffered messages will be sent on reconnect");
        ConnectionResult::ReconnectImmediate(returned_rx)
    }

    /// Handle server events (commands from server)
    async fn handle_server_event(
        kind: &str,
        data: &Value,
        session_manager: &SessionManager,
        relay_id: &str,
        _buffer_tx: &mpsc::Sender<RelayOutput>,
        setup_script: Option<&str>,
    ) -> Option<RelayOutput> {
        match kind {
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

                let params = todoki_protocol::SpawnSessionParams {
                    agent_id: agent_id.to_string(),
                    session_id: session_id.to_string(),
                    workdir: workdir.to_string(),
                    command: command.to_string(),
                    args,
                    env,
                    setup_script: setup_script.map(|s| s.to_string()),
                };

                match session_manager.spawn(params).await {
                    Ok(_result) => {
                        tracing::info!(
                            request_id = %request_id,
                            session_id = %session_id,
                            "spawn succeeded"
                        );
                        Some(RelayOutput::EmitEvent {
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
                        Some(RelayOutput::EmitEvent {
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
                None
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
        let outcome: EventPermissionOutcome = serde_json::from_value(value.clone()).ok()?;
        match outcome {
            EventPermissionOutcome::Selected { selected } => Some(
                agent_client_protocol::RequestPermissionOutcome::Selected(
                    agent_client_protocol::SelectedPermissionOutcome::new(selected),
                ),
            ),
            EventPermissionOutcome::Cancelled { .. } => {
                Some(agent_client_protocol::RequestPermissionOutcome::Cancelled)
            }
        }
    }
}

enum ConnectionResult {
    /// Reconnect after delay, with the buffer receiver
    Reconnect(mpsc::Receiver<RelayOutput>),
    /// Reconnect immediately (was connected successfully), reset backoff
    ReconnectImmediate(mpsc::Receiver<RelayOutput>),
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
