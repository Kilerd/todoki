use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Execution Mode
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    #[default]
    Local,
    Remote,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Local => "local",
            ExecutionMode::Remote => "remote",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "remote" => ExecutionMode::Remote,
            _ => ExecutionMode::Local,
        }
    }
}

// ============================================================================
// Agent Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    #[default]
    Created,
    Running,
    Stopped,
    Exited,
    Failed,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Created => "created",
            AgentStatus::Running => "running",
            AgentStatus::Stopped => "stopped",
            AgentStatus::Exited => "exited",
            AgentStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => AgentStatus::Running,
            "stopped" => AgentStatus::Stopped,
            "exited" => AgentStatus::Exited,
            "failed" => AgentStatus::Failed,
            _ => AgentStatus::Created,
        }
    }
}

// ============================================================================
// Session Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Running => "running",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => SessionStatus::Running,
            "completed" => SessionStatus::Completed,
            "cancelled" => SessionStatus::Cancelled,
            _ => SessionStatus::Failed,
        }
    }
}

// ============================================================================
// Output Stream
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
    System,
    Acp,
    PermissionRequest,
}

impl OutputStream {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputStream::Stdout => "stdout",
            OutputStream::Stderr => "stderr",
            OutputStream::System => "system",
            OutputStream::Acp => "acp",
            OutputStream::PermissionRequest => "permission_request",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "stdout" => OutputStream::Stdout,
            "stderr" => OutputStream::Stderr,
            "acp" => OutputStream::Acp,
            "permission_request" => OutputStream::PermissionRequest,
            _ => OutputStream::System,
        }
    }
}

// ============================================================================
// Agent Role
// ============================================================================

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
    pub args: String,              // JSON string: ["arg1", "arg2"]
    pub execution_mode: String,    // "local" or "remote"
    pub role: String,              // "general", "business", "coding", "qa"
    pub relay_id: Option<String>,
    pub status: String,            // "created", "running", etc.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    pub fn args_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.args).unwrap_or_default()
    }

    pub fn execution_mode_enum(&self) -> ExecutionMode {
        ExecutionMode::from_str(&self.execution_mode)
    }

    pub fn role_enum(&self) -> AgentRole {
        AgentRole::from_str(&self.role)
    }

    pub fn status_enum(&self) -> AgentStatus {
        AgentStatus::from_str(&self.status)
    }
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgent {
    pub name: String,
    pub workdir: String,
    pub command: String,
    pub args: String,
    pub execution_mode: String,
    pub role: String,
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
        relay_id: Option<String>,
    ) -> Self {
        Self {
            name,
            workdir,
            command,
            args: serde_json::to_string(&args).unwrap_or_else(|_| "[]".to_string()),
            execution_mode: execution_mode.as_str().to_string(),
            role: role.as_str().to_string(),
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
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl AgentSession {
    pub fn status_enum(&self) -> SessionStatus {
        SessionStatus::from_str(&self.status)
    }
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
    pub stream: String,
    pub message: String,
}

impl AgentEvent {
    pub fn stream_enum(&self) -> OutputStream {
        OutputStream::from_str(&self.stream)
    }
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateAgentEvent {
    pub agent_id: Uuid,
    pub session_id: Uuid,
    pub seq: i64,
    pub stream: String,
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
            stream: stream.as_str().to_string(),
            message,
        }
    }
}

// ============================================================================
// API DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: Uuid,
    pub name: String,
    pub workdir: String,
    pub command: String,
    pub args: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub role: AgentRole,
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
            execution_mode: a.execution_mode_enum(),
            role: a.role_enum(),
            relay_id: a.relay_id.clone(),
            status: a.status_enum(),
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            status: s.status_enum(),
            started_at: s.started_at,
            ended_at: s.ended_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            stream: e.stream_enum(),
            message: e.message,
        }
    }
}
