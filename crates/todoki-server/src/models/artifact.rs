use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ============================================================================
// Artifact
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Domain, Schematic)]
#[domain(table = "artifacts")]
pub struct Artifact {
    #[domain(primary_key)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub artifact_type: String,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateArtifact {
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub artifact_type: String,
    pub data: Value,
}

impl CreateArtifact {
    pub fn new(
        task_id: Uuid,
        project_id: Uuid,
        agent_id: Option<Uuid>,
        session_id: Option<Uuid>,
        artifact_type: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            task_id,
            project_id,
            agent_id,
            session_id,
            artifact_type: artifact_type.into(),
            data,
        }
    }
}

// ============================================================================
// Artifact Response
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct ArtifactResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub artifact_type: String,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Artifact> for ArtifactResponse {
    fn from(artifact: Artifact) -> Self {
        Self {
            id: artifact.id,
            task_id: artifact.task_id,
            project_id: artifact.project_id,
            agent_id: artifact.agent_id,
            session_id: artifact.session_id,
            artifact_type: artifact.artifact_type,
            data: artifact.data,
            created_at: artifact.created_at,
            updated_at: artifact.updated_at,
        }
    }
}
