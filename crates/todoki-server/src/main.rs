mod api;
mod auth;
mod config;
mod db;
mod event_bus;
mod models;
mod relay;

use std::ops::Deref;
use std::sync::Arc;

use gotcha::Gotcha;
use gotcha::Json;
use gotcha::axum::extract::FromRef;
use gotcha::axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;
use tracing::{error, info};

use crate::api::{agents, artifacts, projects, relays, report, tasks};
use crate::auth::auth_middleware;
use crate::config::Settings;
use crate::db::DatabaseService;
use crate::relay::{RelayManager, RequestTracker};

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

/// Event Publisher wrapper for state extraction
#[derive(Clone)]
pub struct Publisher(pub Arc<event_bus::EventPublisher>);

impl Deref for Publisher {
    type Target = Arc<event_bus::EventPublisher>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Event Subscriber wrapper for state extraction
#[derive(Clone)]
pub struct Subscriber(pub Arc<event_bus::EventSubscriber>);

impl Deref for Subscriber {
    type Target = Arc<event_bus::EventSubscriber>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Request Tracker wrapper for state extraction
#[derive(Clone)]
pub struct ReqTracker(pub Arc<RequestTracker>);

impl Deref for ReqTracker {
    type Target = Arc<RequestTracker>;

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
    pub event_publisher: Arc<event_bus::EventPublisher>,
    pub event_subscriber: Arc<event_bus::EventSubscriber>,
    pub request_tracker: Arc<RequestTracker>,
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

// Allow extracting Publisher from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Publisher {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        Publisher(ctx.state.event_publisher.clone())
    }
}

// Allow extracting Subscriber from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Subscriber {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        Subscriber(ctx.state.event_subscriber.clone())
    }
}

// Allow extracting ReqTracker from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for ReqTracker {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        ReqTracker(ctx.state.request_tracker.clone())
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

    let db = Db(db_service.clone());
    let relay_manager = Arc::new(RelayManager::new());

    // Initialize Event Bus
    info!("Initializing Event Bus...");
    let event_store = Arc::new(event_bus::PgEventStore::new(db_service.pool()));
    let event_publisher = Arc::new(event_bus::EventPublisher::new(event_store.clone()));
    let event_subscriber = Arc::new(event_bus::EventSubscriber::new(event_store.clone()));

    // Initialize Request Tracker for async request-response pattern
    let request_tracker = Arc::new(RequestTracker::new());

    // Start relay response handler in background
    {
        let publisher = event_publisher.clone();
        let db = db_service.clone();
        let tracker = request_tracker.clone();

        tokio::spawn(async move {
            handle_relay_responses(publisher, db, tracker).await;
        });
        info!("Relay response handler started");
    }

    let app_settings = settings.application.clone();
    let app_state = AppState {
        db: db.clone(),
        settings: app_settings.clone(),
        relays: relay_manager.clone(),
        event_publisher: event_publisher.clone(),
        event_subscriber: event_subscriber.clone(),
        request_tracker: request_tracker.clone(),
    };

    info!("Relay manager initialized");

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
        .get(
            "/api/projects/:project_id/artifacts",
            artifacts::list_artifacts,
        )
        .get("/api/artifacts/:artifact_id", artifacts::get_artifact)
        // Agent routes
        .get("/api/agents", agents::list_agents)
        .post("/api/agents", agents::create_agent)
        .get("/api/agents/:agent_id", agents::get_agent)
        .delete("/api/agents/:agent_id", agents::delete_agent)
        .post("/api/agents/:agent_id/start", agents::start_agent)
        .post("/api/agents/:agent_id/stop", agents::stop_agent)
        .get("/api/agents/:agent_id/sessions", agents::get_agent_sessions)
        // Relay routes
        // Note: Relay WebSocket connections now use /ws/event-bus with relay_id parameter
        .get("/api/relays", relays::list_relays)
        .get("/api/relays/:relay_id", relays::get_relay)
        .get(
            "/api/projects/:project_id/relays",
            relays::list_relays_by_project,
        )
        // Event Bus routes
        .get("/api/event-bus", api::event_bus::query_events)
        .get("/api/event-bus/latest", api::event_bus::get_latest_cursor)
        .post("/api/event-bus/replay", api::event_bus::replay_events)
        .post("/api/event-bus/emit", api::event_bus::emit_event)
        // Event Bus WebSocket (for real-time event streaming)
        .get("/ws/event-bus", api::event_bus_ws::event_bus_websocket)
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

// ============================================================================
// Background Handlers
// ============================================================================

/// Handle relay response events from Event Bus
///
/// Listens for:
/// - relay.spawn_completed: Notifies waiting request trackers
/// - relay.spawn_failed: Notifies waiting request trackers with error
/// - agent.session_exited: Updates session status in database
async fn handle_relay_responses(
    publisher: Arc<event_bus::EventPublisher>,
    db: Arc<DatabaseService>,
    tracker: Arc<RequestTracker>,
) {
    use tokio::sync::broadcast;

    let mut rx = publisher.subscribe();

    loop {
        match rx.recv().await {
            Ok(event) => {
                match event.kind.as_str() {
                    "relay.spawn_completed" => {
                        if let Some(req_id) = event.data.get("request_id").and_then(|v| v.as_str())
                        {
                            let result = Ok(serde_json::json!({
                                "session_id": event.data.get("session_id"),
                            }));
                            tracker.complete_request(req_id, result).await;
                        }

                        // Update session status
                        if let Some(session_id_str) =
                            event.data.get("session_id").and_then(|v| v.as_str())
                        {
                            if let Ok(session_uuid) = uuid::Uuid::parse_str(session_id_str) {
                                if let Err(e) =
                                    db.update_session_status(session_uuid, models::SessionStatus::Running)
                                        .await
                                {
                                    error!(error = %e, session_id = %session_id_str, "failed to update session status");
                                }
                            }
                        }
                    }

                    "relay.spawn_failed" => {
                        if let Some(req_id) = event.data.get("request_id").and_then(|v| v.as_str())
                        {
                            let error_msg = event
                                .data
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown error");
                            tracker
                                .complete_request(req_id, Err(anyhow::anyhow!("{}", error_msg)))
                                .await;
                        }
                    }

                    "agent.session_exited" => {
                        if let Some(session_id_str) =
                            event.data.get("session_id").and_then(|v| v.as_str())
                        {
                            if let Ok(session_uuid) = uuid::Uuid::parse_str(session_id_str) {
                                let status_str = event
                                    .data
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("completed");
                                let status = match status_str {
                                    "completed" => models::SessionStatus::Completed,
                                    "failed" => models::SessionStatus::Failed,
                                    "cancelled" => models::SessionStatus::Cancelled,
                                    _ => models::SessionStatus::Completed,
                                };
                                if let Err(e) =
                                    db.update_session_status(session_uuid, status)
                                        .await
                                {
                                    error!(error = %e, session_id = %session_id_str, "failed to update session status");
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(
                    lagged_events = n,
                    "relay response handler lagged, some events may be missed"
                );
            }
            Err(_) => {
                error!("relay response channel closed");
                break;
            }
        }
    }
}
