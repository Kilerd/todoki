mod api;
mod auth;
mod config;
mod db;
mod models;
mod relay;

use std::ops::Deref;
use std::sync::Arc;

use gotcha::axum::extract::FromRef;
use gotcha::axum::response::{IntoResponse, Response};
use gotcha::Gotcha;
use gotcha::Json;
use serde_json::json;
use thiserror::Error;
use tracing::{error, info};

use crate::api::{agent_stream, agents, artifacts, projects, relays, report, tasks};
use crate::auth::auth_middleware;
use crate::config::Settings;
use crate::db::DatabaseService;
use crate::relay::{AgentBroadcaster, RelayManager};

// ============================================================================
// Database wrapper
// ============================================================================

/// Database service wrapper for state extraction
#[derive(Clone)]
pub struct Db(pub Arc<DatabaseService>);

impl Deref for Db {
    type Target = DatabaseService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Relay manager wrapper for state extraction
#[derive(Clone)]
pub struct Relays(pub Arc<RelayManager>);

impl Deref for Relays {
    type Target = Arc<RelayManager>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Broadcaster wrapper for state extraction
#[derive(Clone)]
pub struct Broadcaster(pub Arc<AgentBroadcaster>);

impl Deref for Broadcaster {
    type Target = AgentBroadcaster;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// Error types
// ============================================================================

#[derive(Error, Debug)]
pub enum TodokiError {
    #[error("Database error: {0}")]
    Database(#[from] conservator::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] conservator::MigrateError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, TodokiError>;

impl TodokiError {
    pub fn to_status_code(&self) -> gotcha::axum::http::StatusCode {
        use gotcha::axum::http::StatusCode;
        match self {
            TodokiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TodokiError::Migration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TodokiError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TodokiError::Auth(_) => StatusCode::UNAUTHORIZED,
            TodokiError::NotFound(_) => StatusCode::NOT_FOUND,
            TodokiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// ============================================================================
// Application state
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub settings: Settings,
    pub relays: Arc<RelayManager>,
    pub broadcaster: Arc<AgentBroadcaster>,
}

impl Default for AppState {
    fn default() -> Self {
        unimplemented!()
    }
}

// Allow extracting Db from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Db {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        ctx.state.db.clone()
    }
}

// Allow extracting Settings from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Settings {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        ctx.state.settings.clone()
    }
}

// Allow extracting Relays from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Relays {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        Relays(ctx.state.relays.clone())
    }
}

// Allow extracting Broadcaster from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Broadcaster {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        Broadcaster(ctx.state.broadcaster.clone())
    }
}

// ============================================================================
// Health check handler
// ============================================================================

async fn health_check() -> Response {
    Json(json!({"status": "ok"})).into_response()
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .init();

    info!("Starting Todoki API Server");

    let settings = Settings::new().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Initializing database...");
    let db_service = Arc::new(DatabaseService::new(&settings.application.database_url)?);

    info!("Running database migrations...");
    db_service.migrate().await?;

    let db = Db(db_service);
    let relay_manager = Arc::new(RelayManager::new());
    let broadcaster = Arc::new(AgentBroadcaster::new());

    let app_settings = settings.application.clone();
    let app_state = AppState {
        db: db.clone(),
        settings: app_settings.clone(),
        relays: relay_manager.clone(),
        broadcaster: broadcaster.clone(),
    };

    info!("Relay manager and broadcaster initialized");

    let addr = format!("{}:{}", &settings.basic.host, &settings.basic.port);
    info!("Starting server on http://{}", addr);

    Gotcha::with_types::<AppState, Settings>()
        .state(app_state)
        .config(settings)
        // Health check
        .get("/api", health_check)
        // Task routes
        .get("/api/tasks", tasks::get_tasks)
        .get("/api/tasks/inbox", tasks::get_inbox_tasks)
        .get("/api/tasks/backlog", tasks::get_backlog_tasks)
        .get("/api/tasks/in-progress", tasks::get_in_progress_tasks)
        .get("/api/tasks/done", tasks::get_done_tasks)
        .get("/api/tasks/done/today", tasks::get_today_done_tasks)
        .post("/api/tasks", tasks::create_task)
        .get("/api/tasks/:task_id", tasks::get_task)
        .put("/api/tasks/:task_id", tasks::update_task)
        .post("/api/tasks/:task_id/status", tasks::update_task_status)
        .post("/api/tasks/:task_id/archive", tasks::archive_task)
        .post("/api/tasks/:task_id/unarchive", tasks::unarchive_task)
        .delete("/api/tasks/:task_id", tasks::delete_task)
        .post("/api/tasks/:task_id/comments", tasks::add_comment)
        .post("/api/tasks/:task_id/execute", tasks::execute_task)
        // Project routes
        .get("/api/projects", projects::list_projects)
        .post("/api/projects", projects::create_project)
        .get("/api/projects/by-name/:name", projects::get_project_by_name)
        .get("/api/projects/:project_id", projects::get_project)
        .put("/api/projects/:project_id", projects::update_project)
        .delete("/api/projects/:project_id", projects::delete_project)
        // Report route
        .get("/api/report", report::get_report)
        // Artifact routes
        .get("/api/projects/:project_id/artifacts", artifacts::list_artifacts)
        .get("/api/artifacts/:artifact_id", artifacts::get_artifact)
        // Agent routes
        .get("/api/agents", agents::list_agents)
        .post("/api/agents", agents::create_agent)
        .get("/api/agents/:agent_id", agents::get_agent)
        .delete("/api/agents/:agent_id", agents::delete_agent)
        .post("/api/agents/:agent_id/start", agents::start_agent)
        .post("/api/agents/:agent_id/stop", agents::stop_agent)
        .get("/api/agents/:agent_id/sessions", agents::get_agent_sessions)
        .get("/api/agents/:agent_id/events", agents::get_agent_events)
        .post("/api/agents/:agent_id/input", agents::send_input)
        .post("/api/agents/:agent_id/permission", agents::respond_permission)
        // Relay routes (before auth middleware so WebSocket can use token in query)
        .get("/ws/relays", relays::ws_relay)
        .get("/api/relays", relays::list_relays)
        .get("/api/relays/:relay_id", relays::get_relay)
        // Agent stream WebSocket (for frontend real-time updates)
        .get("/ws/agents/:agent_id/stream", agent_stream::ws_agent_stream)
        .layer(gotcha::axum::middleware::from_fn_with_state(
            app_settings,
            auth_middleware,
        ))
        .with_cors()
        .with_openapi()
        .listen(addr)
        .await?;

    Ok(())
}
