use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::{RelayInfo, AgentRole};

/// A set of project UUIDs for efficient lookup
pub type ProjectSet = HashSet<Uuid>;

/// Pending permission request info - only stores session_id, relay_id is derived from active_sessions
#[derive(Clone)]
struct PendingPermission {
    session_id: String,
}

/// Relay connection manager (in-memory)
#[derive(Clone)]
pub struct RelayManager {
    /// relay_id -> RelayConnection
    relays: Arc<RwLock<HashMap<String, RelayConnection>>>,
    /// Pending permission requests: request_id -> PendingPermission
    pending_permissions: Arc<Mutex<HashMap<String, PendingPermission>>>,
}

pub struct RelayConnection {
    pub relay_id: String,
    pub name: String,
    pub role: AgentRole,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
    pub projects: ProjectSet,
    pub setup_script: Option<String>,
    pub connected_at: i64,
    pub active_sessions: HashSet<String>,
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            relays: Arc::new(RwLock::new(HashMap::new())),
            pending_permissions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a relay connection with a stable ID provided by the relay
    /// If a relay with the same ID is already connected, it will be replaced (reconnect scenario)
    ///
    /// In the new Event Bus architecture, relays communicate via the Event Bus WebSocket
    /// instead of a dedicated channel. The tx channel has been removed.
    pub async fn register(
        &self,
        relay_id: String,
        name: String,
        role: AgentRole,
        safe_paths: Vec<String>,
        labels: HashMap<String, String>,
        projects: Vec<Uuid>,
        setup_script: Option<String>,
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

        let projects_set: ProjectSet = projects.iter().copied().collect();

        let connection = RelayConnection {
            relay_id: relay_id.clone(),
            name: name.clone(),
            role,
            safe_paths,
            labels,
            projects: projects_set,
            setup_script,
            connected_at: Utc::now().timestamp(),
            active_sessions: previous_sessions,
        };

        relays.insert(relay_id.clone(), connection);
        tracing::info!(
            relay_id = %relay_id,
            name = %name,
            role = ?role,
            projects_count = projects.len(),
            "relay registered"
        );

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

    /// Select an available relay based on role, project and availability
    /// - If preferred_id is specified and the relay is idle, use it
    /// - Otherwise, find an idle relay that matches the required_role (or is General)
    ///   and the required_project (or has no project restrictions)
    pub async fn select_relay(
        &self,
        preferred_id: Option<&str>,
        required_role: Option<AgentRole>,
        required_project: Option<Uuid>,
    ) -> Option<String> {
        let relays = self.relays.read().await;

        // Helper to check if relay matches role requirement
        let role_matches = |conn: &RelayConnection, required: Option<AgentRole>| -> bool {
            match required {
                None => true, // No role requirement, any relay works
                Some(req) => conn.role == req || conn.role == AgentRole::General,
            }
        };

        // Helper to check if relay matches project requirement
        // A relay with empty projects accepts all projects (universal mode)
        let project_matches = |conn: &RelayConnection, required: Option<Uuid>| -> bool {
            match required {
                None => true, // No project requirement, any relay works
                Some(project_id) => conn.projects.is_empty() || conn.projects.contains(&project_id),
            }
        };

        // Helper to check if relay is idle (single-task mode)
        let is_idle = |conn: &RelayConnection| -> bool { conn.active_sessions.is_empty() };

        // Combined check
        let matches_all =
            |conn: &RelayConnection| -> bool { is_idle(conn) && role_matches(conn, required_role) && project_matches(conn, required_project) };

        // If preferred relay is specified, check if it's available
        if let Some(id) = preferred_id {
            if let Some(conn) = relays.get(id) {
                if matches_all(conn) {
                    return Some(id.to_string());
                }
            }
        }

        // Find first idle relay that matches all requirements
        relays.values().find(|conn| matches_all(conn)).map(|conn| conn.relay_id.clone())
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
                role: conn.role.as_str().to_string(),
                safe_paths: conn.safe_paths.clone(),
                labels: conn.labels.clone(),
                projects: conn.projects.iter().copied().collect(),
                setup_script: conn.setup_script.clone(),
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
            role: conn.role.as_str().to_string(),
            safe_paths: conn.safe_paths.clone(),
            labels: conn.labels.clone(),
            projects: conn.projects.iter().copied().collect(),
            setup_script: conn.setup_script.clone(),
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

    /// List connected relays for a specific project
    /// Returns relays that either:
    /// - Have empty projects list (universal relays)
    /// - Have the specified project in their projects list
    pub async fn list_relays_by_project(&self, project_id: Uuid) -> Vec<RelayInfo> {
        let relays = self.relays.read().await;
        relays
            .values()
            .filter(|conn| conn.projects.is_empty() || conn.projects.contains(&project_id))
            .map(|conn| RelayInfo {
                relay_id: conn.relay_id.clone(),
                name: conn.name.clone(),
                role: conn.role.as_str().to_string(),
                safe_paths: conn.safe_paths.clone(),
                labels: conn.labels.clone(),
                projects: conn.projects.iter().copied().collect(),
                setup_script: conn.setup_script.clone(),
                connected_at: conn.connected_at,
                active_session_count: conn.active_sessions.len(),
            })
            .collect()
    }

    /// Get relay_id for a given session_id by searching active_sessions
    pub async fn get_relay_for_session(&self, session_id: &str) -> Option<String> {
        let relays = self.relays.read().await;
        for conn in relays.values() {
            if conn.active_sessions.contains(session_id) {
                return Some(conn.relay_id.clone());
            }
        }
        None
    }

    /// Store a pending permission request
    /// Note: relay_id is kept in signature for logging but not stored (derived from active_sessions)
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

    /// Get pending permission info without removing it
    /// Returns (relay_id, session_id) - relay_id is derived from active_sessions
    pub async fn get_pending_permission(&self, request_id: &str) -> Option<(String, String)> {
        let pending = self.pending_permissions.lock().await;
        if let Some(p) = pending.get(request_id) {
            let session_id = p.session_id.clone();
            drop(pending); // Release lock before calling get_relay_for_session
            if let Some(relay_id) = self.get_relay_for_session(&session_id).await {
                return Some((relay_id, session_id));
            }
        }
        None
    }

    /// Remove a pending permission request
    pub async fn remove_pending_permission(&self, request_id: &str) {
        let mut pending = self.pending_permissions.lock().await;
        pending.remove(request_id);
    }

    /// Emit a relay command event to Event Bus
    ///
    /// This replaces the old RPC-based approach. The relay will receive the event
    /// through its Event Bus WebSocket subscription.
    pub async fn emit_relay_command(
        &self,
        publisher: &crate::event_bus::EventPublisher,
        relay_id: &str,
        kind: &str,
        request_id: String,
        mut data: Value,
    ) -> anyhow::Result<String> {
        // Check if relay is connected
        if !self.is_connected(relay_id).await {
            anyhow::bail!("relay {} not connected", relay_id);
        }

        // Inject relay_id and request_id into event data
        if let Some(obj) = data.as_object_mut() {
            obj.insert("relay_id".to_string(), Value::String(relay_id.to_string()));
            obj.insert(
                "request_id".to_string(),
                Value::String(request_id.clone()),
            );
        }

        // Create and emit event
        let event = crate::event_bus::Event::new(kind.to_string(), Uuid::nil(), data);
        publisher.emit(event).await?;

        tracing::debug!(
            kind = %kind,
            relay_id = %relay_id,
            request_id = %request_id,
            "emitted relay command event"
        );

        Ok(request_id)
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

        // Register a coding relay
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay".to_string(),
                AgentRole::Coding,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // Register a general relay
        manager
            .register(
                "general-1".to_string(),
                "General Relay".to_string(),
                AgentRole::General,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // Select coding relay - should match coding-1 or general-1 (both work)
        let selected = manager
            .select_relay(None, Some(AgentRole::Coding), None)
            .await;
        assert!(
            selected == Some("coding-1".to_string()) || selected == Some("general-1".to_string()),
            "Expected coding-1 or general-1, got {:?}",
            selected
        );

        // Select business relay - should match general-1 (General accepts any)
        let selected = manager
            .select_relay(None, Some(AgentRole::Business), None)
            .await;
        assert_eq!(selected, Some("general-1".to_string()));

        // Select without role - should match any idle relay
        let selected = manager.select_relay(None, None, None).await;
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_select_relay_idle_only() {
        let manager = RelayManager::new();

        // Register a coding relay
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay".to_string(),
                AgentRole::Coding,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // Mark it as busy
        manager.add_active_session("coding-1", "session-1").await;

        // Try to select - should fail (no idle relay)
        let selected = manager
            .select_relay(None, Some(AgentRole::Coding), None)
            .await;
        assert_eq!(selected, None);

        // Remove active session
        manager.remove_active_session("coding-1", "session-1").await;

        // Now selection should succeed
        let selected = manager
            .select_relay(None, Some(AgentRole::Coding), None)
            .await;
        assert_eq!(selected, Some("coding-1".to_string()));
    }

    #[tokio::test]
    async fn test_select_relay_preferred_id() {
        let manager = RelayManager::new();

        // Register two coding relays
        manager
            .register(
                "coding-1".to_string(),
                "Coding Relay 1".to_string(),
                AgentRole::Coding,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        manager
            .register(
                "coding-2".to_string(),
                "Coding Relay 2".to_string(),
                AgentRole::Coding,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // Select with preferred ID
        let selected = manager
            .select_relay(Some("coding-2"), Some(AgentRole::Coding), None)
            .await;
        assert_eq!(selected, Some("coding-2".to_string()));

        // Mark preferred as busy, should fall back to other
        manager.add_active_session("coding-2", "session-1").await;
        let selected = manager
            .select_relay(Some("coding-2"), Some(AgentRole::Coding), None)
            .await;
        assert_eq!(selected, Some("coding-1".to_string()));
    }

    #[tokio::test]
    async fn test_select_relay_by_project() {
        let manager = RelayManager::new();

        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();

        // Register a relay bound to project_a
        manager
            .register(
                "relay-a".to_string(),
                "Relay A".to_string(),
                AgentRole::General,
                vec![],
                HashMap::new(),
                vec![project_a],
                None,
            )
            .await;

        // Register a universal relay (empty projects)
        manager
            .register(
                "relay-universal".to_string(),
                "Universal Relay".to_string(),
                AgentRole::General,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // Select for project_a - relay-a should match
        let selected = manager
            .select_relay(None, None, Some(project_a))
            .await;
        assert!(
            selected == Some("relay-a".to_string()) || selected == Some("relay-universal".to_string()),
            "Expected relay-a or relay-universal for project_a, got {:?}",
            selected
        );

        // Mark relay-a as busy
        manager.add_active_session("relay-a", "session-1").await;

        // Now only relay-universal should match for project_a
        let selected = manager
            .select_relay(None, None, Some(project_a))
            .await;
        assert_eq!(selected, Some("relay-universal".to_string()));

        // Select for project_b - only relay-universal should match (relay-a is bound to project_a)
        let selected = manager
            .select_relay(None, None, Some(project_b))
            .await;
        assert_eq!(selected, Some("relay-universal".to_string()));

        // Mark relay-universal as busy
        manager.add_active_session("relay-universal", "session-2").await;

        // Now no relay should match for project_b
        let selected = manager
            .select_relay(None, None, Some(project_b))
            .await;
        assert_eq!(selected, None);
    }

    #[tokio::test]
    async fn test_list_relays_by_project() {
        let manager = RelayManager::new();

        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();

        // Register a relay bound to project_a
        manager
            .register(
                "relay-a".to_string(),
                "Relay A".to_string(),
                AgentRole::General,
                vec![],
                HashMap::new(),
                vec![project_a],
                None,
            )
            .await;

        // Register a relay bound to project_b
        manager
            .register(
                "relay-b".to_string(),
                "Relay B".to_string(),
                AgentRole::Coding,
                vec![],
                HashMap::new(),
                vec![project_b],
                None,
            )
            .await;

        // Register a universal relay (empty projects)
        manager
            .register(
                "relay-universal".to_string(),
                "Universal Relay".to_string(),
                AgentRole::General,
                vec![],
                HashMap::new(),
                vec![],
                None,
            )
            .await;

        // List relays for project_a - should return relay-a and relay-universal
        let relays_for_a = manager.list_relays_by_project(project_a).await;
        assert_eq!(relays_for_a.len(), 2);
        let relay_ids: Vec<&str> = relays_for_a.iter().map(|r| r.relay_id.as_str()).collect();
        assert!(relay_ids.contains(&"relay-a"));
        assert!(relay_ids.contains(&"relay-universal"));

        // List relays for project_b - should return relay-b and relay-universal
        let relays_for_b = manager.list_relays_by_project(project_b).await;
        assert_eq!(relays_for_b.len(), 2);
        let relay_ids: Vec<&str> = relays_for_b.iter().map(|r| r.relay_id.as_str()).collect();
        assert!(relay_ids.contains(&"relay-b"));
        assert!(relay_ids.contains(&"relay-universal"));

        // Test with a project that has no specific relays - should return only universal relay
        let project_c = Uuid::new_v4();
        let relays_for_c = manager.list_relays_by_project(project_c).await;
        assert_eq!(relays_for_c.len(), 1);
        assert_eq!(relays_for_c[0].relay_id, "relay-universal");
    }
}
