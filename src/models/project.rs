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
    // Execution templates for different agent roles
    pub general_template: Option<String>,
    pub business_template: Option<String>,
    pub coding_template: Option<String>,
    pub qa_template: Option<String>,
}

#[derive(Debug, Clone, Creatable)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub general_template: Option<String>,
    pub business_template: Option<String>,
    pub coding_template: Option<String>,
    pub qa_template: Option<String>,
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
            general_template: None,
            business_template: None,
            coding_template: None,
            qa_template: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_template: Option<String>,
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
            general_template: p.general_template,
            business_template: p.business_template,
            coding_template: p.coding_template,
            qa_template: p.qa_template,
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
    pub general_template: Option<String>,
    pub business_template: Option<String>,
    pub coding_template: Option<String>,
    pub qa_template: Option<String>,
}
