use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain, TextEnum};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Execution Mode
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Schematic, TextEnum)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    #[default]
    Local,
    Remote,
}

// ============================================================================
// Agent Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Schematic, TextEnum)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    #[default]
    Created,
    Running,
    Stopped,
    Exited,
    Failed,
}

// ============================================================================
// Session Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schematic, TextEnum)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ============================================================================
// Agent Role
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Schematic, TextEnum)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl From<AgentRole> for todoki_protocol::AgentRole {
    fn from(role: AgentRole) -> Self {
        match role {
            AgentRole::General => todoki_protocol::AgentRole::General,
            AgentRole::Business => todoki_protocol::AgentRole::Business,
            AgentRole::Coding => todoki_protocol::AgentRole::Coding,
            AgentRole::Qa => todoki_protocol::AgentRole::Qa,
        }
    }
}

// ============================================================================
// Agent (Database Model)
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "agents")]
pub struct Agent {
    #[domain(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub workdir: String,
    pub command: String,
    pub args: String,
    pub execution_mode: ExecutionMode,
    pub role: AgentRole,
    pub project_id: Uuid,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    pub fn args_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.args).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgent {
    pub name: String,
    pub workdir: String,
    pub command: String,
    pub args: String,
    pub execution_mode: ExecutionMode,
    pub role: AgentRole,
    pub project_id: Uuid,
}

impl CreateAgent {
    pub fn new(
        name: String,
        workdir: String,
        command: String,
        args: Vec<String>,
        execution_mode: ExecutionMode,
        role: AgentRole,
        project_id: Uuid,
    ) -> Self {
        Self {
            name,
            workdir,
            command,
            args: serde_json::to_string(&args).unwrap_or_else(|_| "[]".to_string()),
            execution_mode,
            role,
            project_id,
        }
    }
}

// ============================================================================
// Agent Session (Database Model)
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "agent_sessions")]
pub struct AgentSession {
    #[domain(primary_key)]
    pub id: Uuid,
    pub agent_id: Uuid,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgentSession {
    pub agent_id: Uuid,
}

// ============================================================================
// API DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct AgentResponse {
    pub id: Uuid,
    pub name: String,
    pub workdir: String,
    pub command: String,
    pub args: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub role: AgentRole,
    pub project_id: Uuid,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Agent> for AgentResponse {
    fn from(a: Agent) -> Self {
        Self {
            id: a.id,
            name: a.name.clone(),
            workdir: a.workdir.clone(),
            command: a.command.clone(),
            args: a.args_vec(),
            execution_mode: a.execution_mode,
            role: a.role,
            project_id: a.project_id,
            status: a.status,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct AgentSessionResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl From<AgentSession> for AgentSessionResponse {
    fn from(s: AgentSession) -> Self {
        Self {
            id: s.id,
            agent_id: s.agent_id,
            status: s.status,
            started_at: s.started_at,
            ended_at: s.ended_at,
        }
    }
}

/// Brief agent info for embedding in other responses (e.g., TaskResponse)
#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct AgentBriefResponse {
    pub id: Uuid,
    pub name: String,
    pub status: AgentStatus,
    pub role: AgentRole,
}

impl From<Agent> for AgentBriefResponse {
    fn from(a: Agent) -> Self {
        Self {
            id: a.id,
            name: a.name.clone(),
            status: a.status,
            role: a.role,
        }
    }
}
