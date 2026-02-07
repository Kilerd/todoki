use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::RelayConfig;
use crate::protocol::{
    PermissionOutcome, RelayToServer, RpcResult, SendInputParams, ServerToRelay,
    SpawnSessionParams, StopSessionParams,
};
use crate::session::SessionManager;

pub struct Relay {
    config: RelayConfig,
    relay_id: Option<String>,
}

impl Relay {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            relay_id: None,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let url = self.config.server_url()?;
        tracing::info!(url = %url, "connecting to server");

        // Add token to URL if configured
        let connect_url = if let Some(token) = &self.config.server.token {
            if url.contains('?') {
                format!("{}&token={}", url, token)
            } else {
                format!("{}?token={}", url, token)
            }
        } else {
            url.clone()
        };

        let (ws_stream, _) = connect_async(&connect_url).await?;
        tracing::info!("connected to server");

        let (mut write, mut read) = ws_stream.split();

        // Channel for outbound messages
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<RelayToServer>(256);

        // Create session manager
        let session_manager = Arc::new(SessionManager::new(
            outbound_tx.clone(),
            self.config.safe_paths().to_vec(),
        ));

        // Send registration
        let register_msg = RelayToServer::Register {
            name: self.config.relay_name(),
            safe_paths: self.config.safe_paths().to_vec(),
            labels: self.config.labels().clone(),
        };
        let msg_text = serde_json::to_string(&register_msg)?;
        write.send(Message::Text(msg_text)).await?;
        tracing::info!(name = %self.config.relay_name(), "registration sent");

        // Spawn outbound sender task
        let outbound_handle = tokio::spawn(async move {
            while let Some(msg) = outbound_rx.recv().await {
                let msg_text = match serde_json::to_string(&msg) {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!(error = %e, "failed to serialize message");
                        continue;
                    }
                };
                if write.send(Message::Text(msg_text)).await.is_err() {
                    break;
                }
            }
        });

        // Process inbound messages
        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(Message::Text(text)) => text,
                Ok(Message::Ping(_)) => {
                    let _ = outbound_tx
                        .send(RelayToServer::Pong)
                        .await;
                    continue;
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("server closed connection");
                    break;
                }
                Ok(_) => continue,
                Err(e) => {
                    tracing::error!(error = %e, "websocket error");
                    break;
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
                    self.relay_id = Some(relay_id.clone());
                    tracing::info!(relay_id = %relay_id, "registered with server");
                }

                ServerToRelay::RpcRequest { id, method, params } => {
                    tracing::debug!(
                        request_id = %id,
                        method = %method,
                        params = %params,
                        "received RPC request from server"
                    );
                    let result =
                        self.handle_rpc(&method, params, session_manager.clone()).await;
                    tracing::debug!(
                        request_id = %id,
                        result = ?result,
                        "RPC handler returned, sending response"
                    );
                    let response = RelayToServer::RpcResponse { id: id.clone(), result };
                    if let Err(e) = outbound_tx.send(response).await {
                        tracing::error!(
                            request_id = %id,
                            error = %e,
                            "failed to send RPC response"
                        );
                    } else {
                        tracing::debug!(request_id = %id, "RPC response sent");
                    }
                }

                ServerToRelay::PermissionResponse {
                    request_id,
                    session_id,
                    outcome,
                } => {
                    tracing::debug!(
                        session_id = %session_id,
                        request_id = %request_id,
                        "received permission response"
                    );

                    let acp_outcome = match outcome {
                        PermissionOutcome::Selected { option_id } => {
                            agent_client_protocol::RequestPermissionOutcome::Selected(
                                agent_client_protocol::SelectedPermissionOutcome::new(option_id),
                            )
                        }
                        PermissionOutcome::Cancelled => {
                            agent_client_protocol::RequestPermissionOutcome::Cancelled
                        }
                    };

                    if let Err(e) = session_manager
                        .respond_permission(&session_id, request_id, acp_outcome)
                        .await
                    {
                        tracing::error!(error = %e, "failed to respond to permission");
                    }
                }

                ServerToRelay::Ping => {
                    let _ = outbound_tx.send(RelayToServer::Pong).await;
                }
            }
        }

        // Cleanup: stop all sessions
        tracing::info!("stopping all sessions");
        session_manager.stop_all().await;

        outbound_handle.abort();
        Ok(())
    }

    async fn handle_rpc(
        &self,
        method: &str,
        params: Value,
        session_manager: Arc<SessionManager>,
    ) -> RpcResult {
        tracing::debug!(method = %method, "handling RPC");

        match method {
            "spawn-session" => {
                let params: SpawnSessionParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => return RpcResult::error(format!("invalid params: {}", e)),
                };

                tracing::info!(
                    session_id = %params.session_id,
                    agent_id = %params.agent_id,
                    command = %params.command,
                    workdir = %params.workdir,
                    "spawning session"
                );

                match session_manager.spawn(params).await {
                    Ok(result) => RpcResult::success(result),
                    Err(e) => {
                        tracing::error!(error = %e, "spawn failed");
                        RpcResult::error(e.to_string())
                    }
                }
            }

            "send-input" => {
                let params: SendInputParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => return RpcResult::error(format!("invalid params: {}", e)),
                };

                match session_manager.send_input(params).await {
                    Ok(()) => RpcResult::success(serde_json::json!({})),
                    Err(e) => RpcResult::error(e.to_string()),
                }
            }

            "stop-session" => {
                let params: StopSessionParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => return RpcResult::error(format!("invalid params: {}", e)),
                };

                tracing::info!(session_id = %params.session_id, "stopping session");

                match session_manager.stop(&params.session_id).await {
                    Ok(()) => RpcResult::success(serde_json::json!({})),
                    Err(e) => RpcResult::error(e.to_string()),
                }
            }

            "close-stdin" => {
                let params: StopSessionParams = match serde_json::from_value(params) {
                    Ok(p) => p,
                    Err(e) => return RpcResult::error(format!("invalid params: {}", e)),
                };

                tracing::debug!(session_id = %params.session_id, "closing stdin");

                match session_manager.close_stdin(&params.session_id).await {
                    Ok(()) => RpcResult::success(serde_json::json!({})),
                    Err(e) => RpcResult::error(e.to_string()),
                }
            }

            _ => {
                tracing::warn!(method = %method, "unknown RPC method");
                RpcResult::error(format!("unknown method: {}", method))
            }
        }
    }
}
