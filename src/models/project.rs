use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Project
// ============================================================================

#[derive(Debug, Clone, Domain)]
#[domain(table = "projects")]
pub struct Project {
    #[domain(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CreateProject {
    pub fn new(name: String, description: Option<String>, color: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            name,
            description,
            color: color.unwrap_or_else(|| "#6B7280".to_string()),
            archived: false,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// API DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Project> for ProjectResponse {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            color: p.color,
            archived: p.archived,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct ProjectCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct ProjectUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub archived: Option<bool>,
}
