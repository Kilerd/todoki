use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use uuid::Uuid;

use super::{PermissionOutcome, RelayInfo, RelayRole, RpcResponse, RpcResult, ServerToRelay};

/// Pending permission request info
#[derive(Clone)]
struct PendingPermission {
    relay_id: String,
    session_id: String,
}

/// Relay connection manager (in-memory)
#[derive(Clone)]
pub struct RelayManager {
    /// relay_id -> RelayConnection
    relays: Arc<RwLock<HashMap<String, RelayConnection>>>,
    /// Pending RPC requests: request_id -> oneshot::Sender
    pending_rpcs: Arc<Mutex<HashMap<String, oneshot::Sender<RpcResponse>>>>,
    /// Pending permission requests: request_id -> PendingPermission
    pending_permissions: Arc<Mutex<HashMap<String, PendingPermission>>>,
}

pub struct RelayConnection {
    pub relay_id: String,
    pub name: String,
    pub role: RelayRole,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
    pub tx: mpsc::Sender<ServerToRelay>,
    pub connected_at: i64,
    pub active_sessions: HashSet<String>,
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            relays: Arc::new(RwLock::new(HashMap::new())),
            pending_rpcs: Arc::new(Mutex::new(HashMap::new())),
            pending_permissions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a relay connection with a stable ID provided by the relay
    /// If a relay with the same ID is already connected, it will be replaced (reconnect scenario)
    pub async fn register(
        &self,
        relay_id: String,
        name: String,
        role: RelayRole,
        safe_paths: Vec<String>,
        labels: HashMap<String, String>,
        tx: mpsc::Sender<ServerToRelay>,
    ) -> String {
        let mut relays = self.relays.write().await;

        // Check if this relay was previously connected (reconnect scenario)
        let previous_sessions = if let Some(old_conn) = relays.remove(&relay_id) {
            tracing::info!(
                relay_id = %relay_id,
                name = %old_conn.name,
                "relay reconnecting, replacing old connection"
            );
            // Preserve active sessions from old connection
            old_conn.active_sessions
        } else {
            HashSet::new()
        };

        let connection = RelayConnection {
            relay_id: relay_id.clone(),
            name: name.clone(),
            role,
            safe_paths,
            labels,
            tx,
            connected_at: Utc::now().timestamp(),
            active_sessions: previous_sessions,
        };

        relays.insert(relay_id.clone(), connection);
        tracing::info!(relay_id = %relay_id, name = %name, role = ?role, "relay registered");

        relay_id
    }

    /// Unregister a relay (on disconnect)
    pub async fn unregister(&self, relay_id: &str) -> Vec<String> {
        let mut relays = self.relays.write().await;
        let active_sessions = if let Some(conn) = relays.remove(relay_id) {
            tracing::info!(
                relay_id = %relay_id,
                name = %conn.name,
                sessions = ?conn.active_sessions,
                "relay unregistered"
            );
            conn.active_sessions.into_iter().collect()
        } else {
            Vec::new()
        };
        active_sessions
    }

    /// Send RPC request to relay and wait for response
    pub async fn call(
        &self,
        relay_id: &str,
        method: &str,
        params: Value,
    ) -> anyhow::Result<Value> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        tracing::debug!(
            request_id = %request_id,
            relay_id = %relay_id,
            method = %method,
            "sending RPC request"
        );

        // Store the pending request
        {
            let mut pending = self.pending_rpcs.lock().await;
            pending.insert(request_id.clone(), tx);
            tracing::debug!(
                request_id = %request_id,
                pending_count = pending.len(),
                "stored pending RPC request"
            );
        }

        // Get relay sender
        let relay_tx = {
            let relays = self.relays.read().await;
            relays
                .get(relay_id)
                .map(|c| c.tx.clone())
                .ok_or_else(|| anyhow::anyhow!("relay not connected"))?
        };

        // Send RPC request
        let msg = ServerToRelay::RpcRequest {
            id: request_id.clone(),
            method: method.to_string(),
            params: params.clone(),
        };

        tracing::debug!(
            request_id = %request_id,
            params = %params,
            "sending RPC message to relay channel"
        );

        if relay_tx.send(msg).await.is_err() {
            tracing::error!(request_id = %request_id, "relay channel send failed");
            let mut pending = self.pending_rpcs.lock().await;
            pending.remove(&request_id);
            anyhow::bail!("relay connection closed");
        }

        tracing::debug!(request_id = %request_id, "RPC message sent, waiting for response");

        // Wait for response with timeout
        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    "RPC timeout after 30s"
                );
                // Clean up pending request on timeout
                let pending = self.pending_rpcs.clone();
                let req_id = request_id.clone();
                tokio::spawn(async move {
                    let mut pending = pending.lock().await;
                    pending.remove(&req_id);
                });
                anyhow::anyhow!("rpc timeout")
            })?
            .map_err(|_| anyhow::anyhow!("rpc cancelled"))?;

        tracing::debug!(
            request_id = %request_id,
            success = response.success,
            "received RPC response"
        );

        if response.success {
            Ok(response.result.unwrap_or(Value::Null))
        } else {
            Err(anyhow::anyhow!(
                "{}",
                response.error.unwrap_or_else(|| "unknown error".to_string())
            ))
        }
    }

    /// Handle RPC response from relay
    pub async fn handle_rpc_response(&self, request_id: &str, result: RpcResult) {
        tracing::debug!(
            request_id = %request_id,
            result = ?result,
            "received RPC response from relay"
        );
        let mut pending = self.pending_rpcs.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            tracing::debug!(request_id = %request_id, "forwarding response to caller");
            let _ = tx.send(result.into());
        } else {
            tracing::warn!(
                request_id = %request_id,
                "no pending request found for RPC response"
            );
        }
    }

    /// Select an available relay based on role and availability
    /// - If preferred_id is specified and the relay is idle, use it
    /// - Otherwise, find an idle relay that matches the required_role (or is General)
    pub async fn select_relay(
        &self,
        preferred_id: Option<&str>,
        required_role: Option<RelayRole>,
    ) -> Option<String> {
        let relays = self.relays.read().await;

        // Helper to check if relay matches role requirement
        let role_matches = |conn: &RelayConnection, required: Option<RelayRole>| -> bool {
            match required {
                None => true, // No role requirement, any relay works
                Some(req) => conn.role == req || conn.role == RelayRole::General,
            }
        };

        // Helper to check if relay is idle (single-task mode)
        let is_idle = |conn: &RelayConnection| -> bool { conn.active_sessions.is_empty() };

        // If preferred relay is specified, check if it's available
        if let Some(id) = preferred_id {
            if let Some(conn) = relays.get(id) {
                if is_idle(conn) && role_matches(conn, required_role) {
                    return Some(id.to_string());
                }
            }
        }

        // Find first idle relay that matches the role
        relays
            .values()
            .find(|conn| is_idle(conn) && role_matches(conn, required_role))
            .map(|conn| conn.relay_id.clone())
    }

    /// Add active session to relay
    pub async fn add_active_session(&self, relay_id: &str, session_id: &str) {
        let mut relays = self.relays.write().await;
        if let Some(conn) = relays.get_mut(relay_id) {
            conn.active_sessions.insert(session_id.to_string());
        }
    }

    /// Remove active session from relay
    pub async fn remove_active_session(&self, relay_id: &str, session_id: &str) {
        let mut relays = self.relays.write().await;
        if let Some(conn) = relays.get_mut(relay_id) {
            conn.active_sessions.remove(session_id);
        }
    }

    /// List all connected relays
    pub async fn list_relays(&self) -> Vec<RelayInfo> {
        let relays = self.relays.read().await;
        relays
            .values()
            .map(|conn| RelayInfo {
                relay_id: conn.relay_id.clone(),
                name: conn.name.clone(),
                role: conn.role,
                safe_paths: conn.safe_paths.clone(),
                labels: conn.labels.clone(),
                connected_at: conn.connected_at,
                active_session_count: conn.active_sessions.len(),
            })
            .collect()
    }

    /// Get relay info by ID
    pub async fn get_relay(&self, relay_id: &str) -> Option<RelayInfo> {
        let relays = self.relays.read().await;
        relays.get(relay_id).map(|conn| RelayInfo {
            relay_id: conn.relay_id.clone(),
            name: conn.name.clone(),
            role: conn.role,
            safe_paths: conn.safe_paths.clone(),
            labels: conn.labels.clone(),
            connected_at: conn.connected_at,
            active_session_count: conn.active_sessions.len(),
        })
    }

    /// Check if relay is connected
    pub async fn is_connected(&self, relay_id: &str) -> bool {
        let relays = self.relays.read().await;
        relays.contains_key(relay_id)
    }

    /// Get count of connected relays
    pub async fn relay_count(&self) -> usize {
        let relays = self.relays.read().await;
        relays.len()
    }

    /// Store a pending permission request
    pub async fn store_permission_request(
        &self,
        relay_id: &str,
        request_id: &str,
        session_id: &str,
    ) {
        let mut pending = self.pending_permissions.lock().await;
        pending.insert(
            request_id.to_string(),
            PendingPermission {
                relay_id: relay_id.to_string(),
                session_id: session_id.to_string(),
            },
        );
        tracing::debug!(
            request_id = %request_id,
            relay_id = %relay_id,
            session_id = %session_id,
            "stored pending permission request"
        );
    }

    /// Respond to a pending permission request
    pub async fn respond_to_permission(
        &self,
        request_id: &str,
        outcome: PermissionOutcome,
    ) -> anyhow::Result<()> {
        // Get and remove the pending permission
        let pending = {
            let mut pending = self.pending_permissions.lock().await;
            pending.remove(request_id)
        };

        let pending = pending.ok_or_else(|| {
            anyhow::anyhow!("permission request not found: {}", request_id)
        })?;

        // Get relay sender
        let relay_tx = {
            let relays = self.relays.read().await;
            relays
                .get(&pending.relay_id)
                .map(|c| c.tx.clone())
                .ok_or_else(|| anyhow::anyhow!("relay not connected"))?
        };

        // Send permission response to relay
        let msg = ServerToRelay::PermissionResponse {
            request_id: request_id.to_string(),
            session_id: pending.session_id,
            outcome,
        };

        relay_tx
            .send(msg)
            .await
            .map_err(|_| anyhow::anyhow!("relay connection closed"))?;

        tracing::debug!(
            request_id = %request_id,
            relay_id = %pending.relay_id,
            "sent permission response"
        );

        Ok(())
    }
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_select_relay_by_role() {
        let manager = RelayManager::new();
        let (tx, _rx) = mpsc::channel(1);

        // Register a coding relay
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay".to_string(),
                RelayRole::Coding,
                vec![],
                HashMap::new(),
                tx.clone(),
            )
            .await;

        // Register a general relay
        manager
            .register(
                "general-1".to_string(),
                "General Relay".to_string(),
                RelayRole::General,
                vec![],
                HashMap::new(),
                tx.clone(),
            )
            .await;

        // Select coding relay - should match coding-1 or general-1 (both work)
        let selected = manager
            .select_relay(None, Some(RelayRole::Coding))
            .await;
        assert!(
            selected == Some("coding-1".to_string()) || selected == Some("general-1".to_string()),
            "Expected coding-1 or general-1, got {:?}",
            selected
        );

        // Select business relay - should match general-1 (General accepts any)
        let selected = manager
            .select_relay(None, Some(RelayRole::Business))
            .await;
        assert_eq!(selected, Some("general-1".to_string()));

        // Select without role - should match any idle relay
        let selected = manager.select_relay(None, None).await;
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_select_relay_idle_only() {
        let manager = RelayManager::new();
        let (tx, _rx) = mpsc::channel(1);

        // Register a coding relay
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay".to_string(),
                RelayRole::Coding,
                vec![],
                HashMap::new(),
                tx.clone(),
            )
            .await;

        // Mark it as busy
        manager.add_active_session("coding-1", "session-1").await;

        // Try to select - should fail (no idle relay)
        let selected = manager
            .select_relay(None, Some(RelayRole::Coding))
            .await;
        assert_eq!(selected, None);

        // Remove active session
        manager.remove_active_session("coding-1", "session-1").await;

        // Now selection should succeed
        let selected = manager
            .select_relay(None, Some(RelayRole::Coding))
            .await;
        assert_eq!(selected, Some("coding-1".to_string()));
    }

    #[tokio::test]
    async fn test_select_relay_preferred_id() {
        let manager = RelayManager::new();
        let (tx, _rx) = mpsc::channel(1);

        // Register two coding relays
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay 1".to_string(),
                RelayRole::Coding,
                vec![],
                HashMap::new(),
                tx.clone(),
            )
            .await;

        manager
            .register(
                "coding-2".to_string(),
                "Coding Relay 2".to_string(),
                RelayRole::Coding,
                vec![],
                HashMap::new(),
                tx.clone(),
            )
            .await;

        // Select with preferred ID
        let selected = manager
            .select_relay(Some("coding-2"), Some(RelayRole::Coding))
            .await;
        assert_eq!(selected, Some("coding-2".to_string()));

        // Mark preferred as busy, should fall back to other
        manager.add_active_session("coding-2", "session-1").await;
        let selected = manager
            .select_relay(Some("coding-2"), Some(RelayRole::Coding))
            .await;
        assert_eq!(selected, Some("coding-1".to_string()));
    }
}
