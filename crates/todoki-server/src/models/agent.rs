use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain, TextEnum};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use specta::{Type};
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
// Output Stream
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schematic, TextEnum, Type)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
    System,
    Acp,
    PermissionRequest,
    PermissionResponse,
}

impl std::str::FromStr for OutputStream {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdout" => Ok(OutputStream::Stdout),
            "stderr" => Ok(OutputStream::Stderr),
            "system" => Ok(OutputStream::System),
            "acp" => Ok(OutputStream::Acp),
            "permission_request" => Ok(OutputStream::PermissionRequest),
            "permission_response" => Ok(OutputStream::PermissionResponse),
            _ => Err(()),
        }
    }
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
    pub relay_id: Option<String>,
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
    pub relay_id: Option<String>,
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
        relay_id: Option<String>,
    ) -> Self {
        Self {
            name,
            workdir,
            command,
            args: serde_json::to_string(&args).unwrap_or_else(|_| "[]".to_string()),
            execution_mode,
            role,
            project_id,
            relay_id,
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
    pub relay_id: Option<String>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgentSession {
    pub agent_id: Uuid,
    pub relay_id: Option<String>,
}

// ============================================================================
// Agent Event (Database Model)
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "agent_events")]
pub struct AgentEvent {
    #[domain(primary_key)]
    pub id: i64,
    pub agent_id: Uuid,
    pub session_id: Uuid,
    pub seq: i64,
    pub ts: DateTime<Utc>,
    pub stream: OutputStream,
    pub message: String,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgentEvent {
    pub agent_id: Uuid,
    pub session_id: Uuid,
    pub seq: i64,
    pub stream: OutputStream,
    pub message: String,
}

impl CreateAgentEvent {
    pub fn new(
        agent_id: Uuid,
        session_id: Uuid,
        seq: i64,
        stream: OutputStream,
        message: String,
    ) -> Self {
        Self {
            agent_id,
            session_id,
            seq,
            stream,
            message,
        }
    }
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
    pub relay_id: Option<String>,
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
            relay_id: a.relay_id.clone(),
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
    pub relay_id: Option<String>,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl From<AgentSession> for AgentSessionResponse {
    fn from(s: AgentSession) -> Self {
        Self {
            id: s.id,
            agent_id: s.agent_id,
            relay_id: s.relay_id.clone(),
            status: s.status,
            started_at: s.started_at,
            ended_at: s.ended_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct AgentEventResponse {
    pub seq: i64,
    pub ts: DateTime<Utc>,
    pub stream: OutputStream,
    pub message: String,
}

impl From<AgentEvent> for AgentEventResponse {
    fn from(e: AgentEvent) -> Self {
        Self {
            seq: e.seq,
            ts: e.ts,
            stream: e.stream,
            message: e.message,
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
