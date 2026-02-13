use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain, TextEnum};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::agent::AgentBriefResponse;
use super::artifact::ArtifactResponse;
use super::project::ProjectResponse;

// ============================================================================
// Task Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schematic, TextEnum, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    #[default]
    Backlog,
    Todo,
    InProgress,
    InReview,
    Done,
}

// ============================================================================
// Task Event Type
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schematic, TextEnum)]
pub enum TaskEventType {
    Create,
    StatusChange,
    Unarchived,
    Archived,
    CreateComment,
}

// ============================================================================
// Task
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "tasks")]
pub struct Task {
    #[domain(primary_key)]
    pub id: Uuid,
    pub priority: i32,
    pub content: String,
    pub project_id: Uuid,
    pub status: TaskStatus,
    pub create_at: DateTime<Utc>,
    pub archived: bool,
    /// Agent ID if this task is being executed by an agent
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateTask {
    pub priority: i32,
    pub content: String,
    pub project_id: Uuid,
    pub status: TaskStatus,
    pub create_at: DateTime<Utc>,
    pub archived: bool,
    pub agent_id: Option<Uuid>,
}

impl CreateTask {
    pub fn new(content: String, status: TaskStatus, priority: i32, project_id: Uuid) -> Self {
        Self {
            priority,
            content,
            project_id,
            status,
            create_at: Utc::now(),
            archived: false,
            agent_id: None,
        }
    }
}

// ============================================================================
// Task Event
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "task_events")]
pub struct TaskEvent {
    #[domain(primary_key)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub datetime: DateTime<Utc>,
    pub state: Option<TaskStatus>,
    pub from_state: Option<TaskStatus>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateTaskEvent {
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub datetime: DateTime<Utc>,
    pub state: Option<TaskStatus>,
    pub from_state: Option<TaskStatus>,
}

impl CreateTaskEvent {
    pub fn create(task_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::Create,
            datetime: Utc::now(),
            state: None,
            from_state: None,
        }
    }

    pub fn status_change(task_id: Uuid, from_status: TaskStatus, to_status: TaskStatus) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::StatusChange,
            datetime: Utc::now(),
            state: Some(to_status),
            from_state: Some(from_status),
        }
    }

    pub fn archived(task_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::Archived,
            datetime: Utc::now(),
            state: None,
            from_state: None,
        }
    }

    pub fn unarchived(task_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::Unarchived,
            datetime: Utc::now(),
            state: None,
            from_state: None,
        }
    }

    pub fn create_comment(task_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::CreateComment,
            datetime: Utc::now(),
            state: None,
            from_state: None,
        }
    }
}

// ============================================================================
// Task Comment
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "task_comments")]
pub struct TaskComment {
    #[domain(primary_key)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub content: String,
    pub create_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateTaskComment {
    pub task_id: Uuid,
    pub content: String,
    pub create_at: DateTime<Utc>,
}

impl CreateTaskComment {
    pub fn new(task_id: Uuid, content: String) -> Self {
        Self {
            task_id,
            content,
            create_at: Utc::now(),
        }
    }
}

// ============================================================================
// API DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct TaskEventResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub datetime: DateTime<Utc>,
    pub state: Option<TaskStatus>,
    pub from_state: Option<TaskStatus>,
}

impl From<TaskEvent> for TaskEventResponse {
    fn from(e: TaskEvent) -> Self {
        Self {
            id: e.id,
            task_id: e.task_id,
            event_type: e.event_type,
            datetime: e.datetime,
            state: e.state,
            from_state: e.from_state,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct TaskCommentResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub content: String,
    pub create_at: DateTime<Utc>,
}

impl From<TaskComment> for TaskCommentResponse {
    fn from(c: TaskComment) -> Self {
        Self {
            id: c.id,
            task_id: c.task_id,
            content: c.content,
            create_at: c.create_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct TaskResponse {
    pub id: Uuid,
    pub priority: i32,
    pub content: String,
    pub project: ProjectResponse,
    pub status: TaskStatus,
    pub create_at: DateTime<Utc>,
    pub archived: bool,
    pub events: Vec<TaskEventResponse>,
    pub comments: Vec<TaskCommentResponse>,
    /// Agent executing this task, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentBriefResponse>,
    /// Artifacts created by the agent (e.g., GitHub PRs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactResponse>,
}

impl TaskResponse {
    pub fn from_task(
        task: Task,
        project: ProjectResponse,
        events: Vec<TaskEvent>,
        comments: Vec<TaskComment>,
        agent: Option<AgentBriefResponse>,
        artifacts: Vec<ArtifactResponse>,
    ) -> Self {
        Self {
            id: task.id,
            priority: task.priority,
            content: task.content,
            project,
            status: task.status,
            create_at: task.create_at,
            archived: task.archived,
            events: events.into_iter().map(Into::into).collect(),
            comments: comments.into_iter().map(Into::into).collect(),
            agent,
            artifacts,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct TaskCreateRequest {
    #[serde(default)]
    pub priority: i32,
    pub content: String,
    pub project_id: Uuid,
    #[serde(default)]
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct TaskUpdateRequest {
    pub priority: i32,
    pub content: String,
    pub project_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct TaskStatusUpdateRequest {
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct TaskCommentCreateRequest {
    pub content: String,
}
