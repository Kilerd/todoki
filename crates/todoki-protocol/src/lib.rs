//! Shared protocol definitions for todoki server and relay communication.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod event_bus;

// Re-export event_bus types for convenience
pub use event_bus::*;

// ============================================================================
// Agent Role
// ============================================================================

/// Agent role for task routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::General => "general",
            AgentRole::Business => "business",
            AgentRole::Coding => "coding",
            AgentRole::Qa => "qa",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "business" => AgentRole::Business,
            "coding" => AgentRole::Coding,
            "qa" => AgentRole::Qa,
            _ => AgentRole::General,
        }
    }
}

// ============================================================================
// Relay Session Parameters (used by relay internally)
// ============================================================================

/// Parameters for spawn-session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionParams {
    pub agent_id: String,
    pub session_id: String,
    pub workdir: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Optional setup script to run before the main command.
    /// If provided, executes as: bash -c "setup_script && exec command args..."
    #[serde(default)]
    pub setup_script: Option<String>,
}

/// Parameters for send-input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInputParams {
    pub session_id: String,
    pub input: String,
}

/// Result for spawn-session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionResult {
    pub pid: u32,
}
