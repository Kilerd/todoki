use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Relay role for task routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RelayRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl RelayRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelayRole::General => "general",
            RelayRole::Business => "business",
            RelayRole::Coding => "coding",
            RelayRole::Qa => "qa",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "business" => RelayRole::Business,
            "coding" => RelayRole::Coding,
            "qa" => RelayRole::Qa,
            _ => RelayRole::General,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub relay: RelaySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// WebSocket URL to connect to (e.g., wss://example.com/ws/relays)
    pub url: Option<String>,
    /// Authentication token
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelaySettings {
    /// Relay name (default: hostname)
    pub name: Option<String>,
    /// Relay role for task routing
    #[serde(default)]
    pub role: RelayRole,
    /// Allowed working directories
    #[serde(default)]
    pub safe_paths: Vec<String>,
    /// Labels for relay selection
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            relay: RelaySettings::default(),
        }
    }
}

impl RelayConfig {
    /// Load config from file and environment variables
    /// Environment variables take precedence over file config
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::load_from_file().unwrap_or_default();

        // Override with environment variables
        if let Ok(url) = std::env::var("TODOKI_SERVER_URL") {
            config.server.url = Some(url);
        }
        if let Ok(token) = std::env::var("TODOKI_RELAY_TOKEN") {
            config.server.token = Some(token);
        }
        if let Ok(name) = std::env::var("TODOKI_RELAY_NAME") {
            config.relay.name = Some(name);
        }
        if let Ok(paths) = std::env::var("TODOKI_SAFE_PATHS") {
            config.relay.safe_paths = paths.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(role) = std::env::var("TODOKI_RELAY_ROLE") {
            config.relay.role = RelayRole::from_str(&role);
        }

        Ok(config)
    }

    fn load_from_file() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: RelayConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".todoki-relay/config.toml")
    }

    /// Get server URL, with fallback
    pub fn server_url(&self) -> anyhow::Result<String> {
        self.server
            .url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("server URL not configured"))
    }

    /// Get relay name, with fallback to hostname
    pub fn relay_name(&self) -> String {
        self.relay
            .name
            .clone()
            .unwrap_or_else(|| hostname::get().map(|h| h.to_string_lossy().to_string()).unwrap_or_else(|_| "relay".to_string()))
    }

    /// Get safe paths
    pub fn safe_paths(&self) -> &[String] {
        &self.relay.safe_paths
    }

    /// Get labels
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.relay.labels
    }

    /// Get relay role
    pub fn role(&self) -> RelayRole {
        self.relay.role
    }
}
