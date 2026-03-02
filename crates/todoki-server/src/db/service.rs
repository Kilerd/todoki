use crate::models::{
    agent::{
        Agent, AgentBriefResponse, AgentSession, AgentStatus, CreateAgent, CreateAgentSession,
        SessionStatus,
    },
    artifact::{Artifact, CreateArtifact},
    project::{CreateProject, Project},
    report::{ReportPeriod, ReportResponse},
    task::{
        CreateTask, CreateTaskComment, CreateTaskEvent, Task, TaskComment, TaskEvent,
        TaskResponse, TaskStatus,
    },
};
use serde_json::Value;
use chrono::Utc;
use conservator::{Creatable, Domain, Executor, Migrator, PooledConnection, SqlTypeWrapper};
use std::sync::Arc;
use uuid::Uuid;

/// Database service for managing all database operations
pub struct DatabaseService {
    pool: Arc<PooledConnection>,
}

impl DatabaseService {
    /// Create a new database service
    pub fn new(database_url: &str) -> crate::Result<Self> {
        let pool =
            PooledConnection::from_url(database_url).map_err(|e| crate::TodokiError::Database(e))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get the underlying connection pool (for Event Bus)
    pub fn pool(&self) -> Arc<PooledConnection> {
        self.pool.clone()
    }

    /// Run database migrations
    pub async fn migrate(&self) -> crate::Result<()> {
        let migrator = Migrator::from_path("./migrations")?;

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        migrator.run(&mut conn).await?;

        tracing::info!("Migrations completed successfully");
        Ok(())
    }

    // ========================================================================
    // Task operations
    // ========================================================================

    /// Get tasks by status (not archived)
    async fn get_tasks_by_status(&self, statuses: &[TaskStatus]) -> crate::Result<Vec<Task>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let status_wrappers: Vec<SqlTypeWrapper<TaskStatus>> =
            statuses.iter().map(|s| SqlTypeWrapper(*s)).collect();
        let placeholders: Vec<String> = (1..=status_wrappers.len())
            .map(|i| format!("${}", i))
            .collect();

        let query = format!(
            r#"
            SELECT id, priority, content, project_id, status, create_at, archived, agent_id
            FROM tasks
            WHERE status IN ({})
              AND archived = false
            ORDER BY priority DESC, create_at DESC
            "#,
            placeholders.join(", ")
        );

        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            status_wrappers.iter().map(|s| s as _).collect();

        let rows = conn
            .query(&query, &params)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Task {
                id: row.get("id"),
                priority: row.get("priority"),
                content: row.get("content"),
                project_id: row.get("project_id"),
                status: row.get::<_, SqlTypeWrapper<TaskStatus>>("status").0,
                create_at: row.get("create_at"),
                archived: row.get("archived"),
                agent_id: row.get("agent_id"),
            })
            .collect())
    }

    /// Get today's tasks (todo, not archived)
    pub async fn get_today_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Todo]).await
    }

    /// Get inbox tasks (todo, in-progress, in-review, not archived)
    pub async fn get_inbox_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Todo, TaskStatus::InProgress, TaskStatus::InReview])
            .await
    }

    /// Get backlog tasks (backlog, not archived)
    pub async fn get_backlog_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Backlog]).await
    }

    /// Get in-progress tasks (any working status, not archived)
    pub async fn get_in_progress_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[
            TaskStatus::InProgress,
            TaskStatus::InReview,
            // Plan phase working states
            TaskStatus::PlanInProgress,
            TaskStatus::PlanReview,
            // Coding phase working states
            TaskStatus::CodingInProgress,
            TaskStatus::CodingReview,
            // Cross-review phase working states
            TaskStatus::CrossReviewInProgress,
        ])
        .await
    }

    /// Get tasks in Plan phase (not archived)
    pub async fn get_plan_phase_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[
            TaskStatus::PlanPending,
            TaskStatus::PlanInProgress,
            TaskStatus::PlanReview,
            TaskStatus::PlanDone,
        ])
        .await
    }

    /// Get tasks in Coding phase (not archived)
    pub async fn get_coding_phase_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[
            TaskStatus::CodingPending,
            TaskStatus::CodingInProgress,
            TaskStatus::CodingReview,
            TaskStatus::CodingDone,
            TaskStatus::InProgress,
            TaskStatus::InReview,
        ])
        .await
    }

    /// Get tasks in Cross-Review phase (not archived)
    pub async fn get_cross_review_phase_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[
            TaskStatus::CrossReviewPending,
            TaskStatus::CrossReviewInProgress,
            TaskStatus::CrossReviewPass,
            TaskStatus::CrossReviewFail,
        ])
        .await
    }

    /// Get done tasks (done, not archived)
    pub async fn get_done_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Done]).await
    }

    /// Get done tasks for a specific project with pagination
    pub async fn get_project_done_tasks(
        &self,
        project_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> crate::Result<Vec<Task>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let query = r#"
            SELECT id, priority, content, project_id, status, create_at, archived, agent_id
            FROM tasks
            WHERE project_id = $1
              AND status = 'done'
              AND archived = false
            ORDER BY create_at DESC
            OFFSET $2
            LIMIT $3
        "#;

        let rows = conn
            .query(query, &[&project_id, &offset, &limit])
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Task {
                id: row.get("id"),
                priority: row.get("priority"),
                content: row.get("content"),
                project_id: row.get("project_id"),
                status: row.get::<_, SqlTypeWrapper<TaskStatus>>("status").0,
                create_at: row.get("create_at"),
                archived: row.get("archived"),
                agent_id: row.get("agent_id"),
            })
            .collect())
    }

    /// Get tasks marked done today (not archived)
    pub async fn get_today_done_tasks(&self) -> crate::Result<Vec<Task>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let query = r#"
            SELECT DISTINCT ON (t.id) t.id, t.priority, t.content, t.project_id, t.status, t.create_at, t.archived, t.agent_id
            FROM tasks t
            JOIN task_events e ON t.id = e.task_id
            WHERE t.status = 'done'
              AND t.archived = false
              AND e.event_type = 'StatusChange'
              AND e.state = 'done'
              AND (e.datetime AT TIME ZONE 'Asia/Hong_Kong')::date = (NOW() AT TIME ZONE 'Asia/Hong_Kong')::date
            ORDER BY t.id, t.priority DESC, t.create_at DESC
        "#;

        let rows = conn
            .query(query, &[])
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Task {
                id: row.get("id"),
                priority: row.get("priority"),
                content: row.get("content"),
                project_id: row.get("project_id"),
                status: row.get::<_, SqlTypeWrapper<TaskStatus>>("status").0,
                create_at: row.get("create_at"),
                archived: row.get("archived"),
                agent_id: row.get("agent_id"),
            })
            .collect())
    }

    /// Get a task by ID
    pub async fn get_task_by_id(&self, task_id: Uuid) -> crate::Result<Option<Task>> {
        match Task::fetch_one_by_pk(&task_id, &*self.pool).await {
            Ok(task) => Ok(Some(task)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::TodokiError::Database(e)),
        }
    }

    /// Get events for a task
    pub async fn get_task_events(&self, task_id: Uuid) -> crate::Result<Vec<TaskEvent>> {
        TaskEvent::select()
            .filter(TaskEvent::COLUMNS.task_id.eq(task_id))
            .order_by(TaskEvent::COLUMNS.datetime.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Get comments for a task
    pub async fn get_task_comments(&self, task_id: Uuid) -> crate::Result<Vec<TaskComment>> {
        TaskComment::select()
            .filter(TaskComment::COLUMNS.task_id.eq(task_id))
            .order_by(TaskComment::COLUMNS.create_at.asc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Get full task response with events, comments, agent info, and artifacts
    pub async fn get_task_response(&self, task: Task) -> crate::Result<TaskResponse> {
        let events = self.get_task_events(task.id).await?;
        let comments = self.get_task_comments(task.id).await?;

        // Load agent info if task has an agent_id
        let agent = if let Some(agent_id) = task.agent_id {
            self.get_agent(agent_id).await?.map(AgentBriefResponse::from)
        } else {
            None
        };

        // Load artifacts for this task
        let artifacts = self
            .list_artifacts_by_task(task.id)
            .await?
            .into_iter()
            .map(crate::models::ArtifactResponse::from)
            .collect();

        Ok(TaskResponse::from_task(task, events, comments, agent, artifacts))
    }

    /// Create a new task
    pub async fn create_task(&self, create_task: CreateTask) -> crate::Result<Task> {
        let task_id = create_task
            .insert::<Task>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        // Create initial event
        let event = CreateTaskEvent::create(task_id);
        let _ = event
            .insert::<TaskEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Update a task
    pub async fn update_task(
        &self,
        task_id: Uuid,
        priority: i32,
        content: String,
        project_id: Uuid,
    ) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.priority = priority;
        task.content = content;
        task.project_id = project_id;

        task.save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(task)
    }

    /// Update task status
    pub async fn update_task_status(
        &self,
        task_id: Uuid,
        new_status: TaskStatus,
    ) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let old_status = task.status;
        task.status = new_status;

        // Create status change event
        let event = CreateTaskEvent::status_change(task_id, old_status, new_status);
        let _ = event
            .insert::<TaskEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(task)
    }

    /// Archive a task
    pub async fn archive_task(&self, task_id: Uuid) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.archived = true;

        // Create archived event
        let event = CreateTaskEvent::archived(task_id);
        let _ = event
            .insert::<TaskEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(task)
    }

    /// Unarchive a task
    pub async fn unarchive_task(&self, task_id: Uuid) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.archived = false;

        // Create unarchived event
        let event = CreateTaskEvent::unarchived(task_id);
        let _ = event
            .insert::<TaskEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(task)
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: Uuid) -> crate::Result<()> {
        Task::delete_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    /// Update task's agent_id (link task to an agent)
    pub async fn update_task_agent_id(
        &self,
        task_id: Uuid,
        agent_id: Option<Uuid>,
    ) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.agent_id = agent_id;

        task.save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(task)
    }

    /// Get task by agent_id (find the task that this agent is executing)
    pub async fn get_task_by_agent_id(&self, agent_id: Uuid) -> crate::Result<Option<Task>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let row = conn
            .query_opt(
                r#"
                SELECT id, priority, content, project_id, status, create_at, archived, agent_id
                FROM tasks
                WHERE agent_id = $1
                LIMIT 1
                "#,
                &[&agent_id],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(row.map(|r| Task {
            id: r.get("id"),
            priority: r.get("priority"),
            content: r.get("content"),
            project_id: r.get("project_id"),
            status: r.get::<_, SqlTypeWrapper<TaskStatus>>("status").0,
            create_at: r.get("create_at"),
            archived: r.get("archived"),
            agent_id: r.get("agent_id"),
        }))
    }

    /// Add a comment to a task
    pub async fn add_task_comment(
        &self,
        task_id: Uuid,
        content: String,
    ) -> crate::Result<TaskComment> {
        // Create comment
        let create_comment = CreateTaskComment::new(task_id, content);
        let comment_id = create_comment
            .insert::<TaskComment>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        // Create comment event
        let event = CreateTaskEvent::create_comment(task_id);
        let _ = event
            .insert::<TaskEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        TaskComment::fetch_one_by_pk(&comment_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    // ========================================================================
    // Project operations
    // ========================================================================

    /// List all projects (not archived by default)
    pub async fn list_projects(&self, include_archived: bool) -> crate::Result<Vec<Project>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let query = if include_archived {
            r#"SELECT id, name, description, color, archived, created_at, updated_at,
                      general_template, business_template, coding_template, qa_template
               FROM projects ORDER BY name ASC"#
        } else {
            r#"SELECT id, name, description, color, archived, created_at, updated_at,
                      general_template, business_template, coding_template, qa_template
               FROM projects WHERE archived = false ORDER BY name ASC"#
        };

        let rows = conn
            .query(query, &[])
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Project {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                color: row.get("color"),
                archived: row.get("archived"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                general_template: row.get("general_template"),
                business_template: row.get("business_template"),
                coding_template: row.get("coding_template"),
                qa_template: row.get("qa_template"),
            })
            .collect())
    }

    /// Get project by ID
    pub async fn get_project(&self, project_id: Uuid) -> crate::Result<Option<Project>> {
        match Project::fetch_one_by_pk(&project_id, &*self.pool).await {
            Ok(project) => Ok(Some(project)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::TodokiError::Database(e)),
        }
    }

    /// Get project by name
    pub async fn get_project_by_name(&self, name: &str) -> crate::Result<Option<Project>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let row = conn
            .query_opt(
                r#"SELECT id, name, description, color, archived, created_at, updated_at,
                          general_template, business_template, coding_template, qa_template
                   FROM projects WHERE name = $1"#,
                &[&name],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(row.map(|r| Project {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            color: r.get("color"),
            archived: r.get("archived"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            general_template: r.get("general_template"),
            business_template: r.get("business_template"),
            coding_template: r.get("coding_template"),
            qa_template: r.get("qa_template"),
        }))
    }

    /// Create a new project
    pub async fn create_project(&self, create: CreateProject) -> crate::Result<Project> {
        let project_id = create
            .insert::<Project>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Project::fetch_one_by_pk(&project_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Update a project
    pub async fn update_project(
        &self,
        project_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        color: Option<String>,
        archived: Option<bool>,
        general_template: Option<String>,
        business_template: Option<String>,
        coding_template: Option<String>,
        qa_template: Option<String>,
    ) -> crate::Result<Project> {
        let mut project = Project::fetch_one_by_pk(&project_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        if let Some(n) = name {
            project.name = n;
        }
        if let Some(d) = description {
            project.description = Some(d);
        }
        if let Some(c) = color {
            project.color = c;
        }
        if let Some(a) = archived {
            project.archived = a;
        }
        if let Some(t) = general_template {
            project.general_template = Some(t);
        }
        if let Some(t) = business_template {
            project.business_template = Some(t);
        }
        if let Some(t) = coding_template {
            project.coding_template = Some(t);
        }
        if let Some(t) = qa_template {
            project.qa_template = Some(t);
        }
        project.updated_at = Utc::now();

        project
            .save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(project)
    }

    /// Delete a project (fails if tasks reference it)
    pub async fn delete_project(&self, project_id: Uuid) -> crate::Result<()> {
        Project::delete_by_pk(&project_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    // ========================================================================
    // Report operations
    // ========================================================================

    /// Get activity report for a given period
    pub async fn get_report(&self, period: ReportPeriod) -> crate::Result<ReportResponse> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let date_filter = match period {
            ReportPeriod::Today => {
                "(datetime AT TIME ZONE 'Asia/Hong_Kong')::date = (NOW() AT TIME ZONE 'Asia/Hong_Kong')::date"
            }
            ReportPeriod::Week => "datetime >= NOW() - INTERVAL '7 days'",
            ReportPeriod::Month => "datetime >= NOW() - INTERVAL '30 days'",
        };

        let query = format!(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = 'Create') AS created_count,
                COUNT(*) FILTER (WHERE event_type = 'StatusChange' AND state = 'done') AS done_count,
                COUNT(*) FILTER (WHERE event_type = 'Archived') AS archived_count,
                COUNT(*) FILTER (WHERE event_type = 'StatusChange') AS state_changes_count,
                COUNT(*) FILTER (WHERE event_type = 'CreateComment') AS comments_count
            FROM task_events
            WHERE {}
            "#,
            date_filter
        );

        let row = conn
            .query_one(&query, &[])
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(ReportResponse {
            period,
            created_count: row.get::<_, Option<i64>>("created_count").unwrap_or(0),
            done_count: row.get::<_, Option<i64>>("done_count").unwrap_or(0),
            archived_count: row.get::<_, Option<i64>>("archived_count").unwrap_or(0),
            state_changes_count: row.get::<_, Option<i64>>("state_changes_count").unwrap_or(0),
            comments_count: row.get::<_, Option<i64>>("comments_count").unwrap_or(0),
        })
    }

    // ========================================================================
    // Agent operations
    // ========================================================================

    /// Create a new agent
    pub async fn create_agent(&self, create: CreateAgent) -> crate::Result<Agent> {
        let agent_id = create
            .insert::<Agent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Agent::fetch_one_by_pk(&agent_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// List all agents
    pub async fn list_agents(&self) -> crate::Result<Vec<Agent>> {
        Agent::fetch_all(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: Uuid) -> crate::Result<Option<Agent>> {
        match Agent::fetch_one_by_pk(&agent_id, &*self.pool).await {
            Ok(agent) => Ok(Some(agent)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::TodokiError::Database(e)),
        }
    }

    /// Update agent status
    pub async fn update_agent_status(
        &self,
        agent_id: Uuid,
        status: AgentStatus,
    ) -> crate::Result<()> {
        let mut agent = Agent::fetch_one_by_pk(&agent_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        agent.status = status;
        agent.updated_at = Utc::now();
        agent
            .save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    /// Delete agent
    pub async fn delete_agent(&self, agent_id: Uuid) -> crate::Result<()> {
        Agent::delete_by_pk(&agent_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    // ========================================================================
    // Agent Session operations
    // ========================================================================

    /// Create a new agent session
    pub async fn create_agent_session(&self, agent_id: Uuid) -> crate::Result<AgentSession> {
        let create = CreateAgentSession { agent_id };

        let session_id = create
            .insert::<AgentSession>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        AgentSession::fetch_one_by_pk(&session_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// Update session status
    pub async fn update_session_status(
        &self,
        session_id: Uuid,
        status: SessionStatus,
    ) -> crate::Result<()> {
        let mut session = AgentSession::fetch_one_by_pk(&session_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        session.status = status;
        if status != SessionStatus::Running {
            session.ended_at = Some(Utc::now());
        }

        session
            .save(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    /// Get a single session by ID
    pub async fn get_agent_session(&self, session_id: Uuid) -> crate::Result<Option<AgentSession>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let row = conn
            .query_opt(
                r#"
                SELECT id, agent_id, status, started_at, ended_at
                FROM agent_sessions
                WHERE id = $1
                "#,
                &[&session_id],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(row.map(|r| AgentSession {
            id: r.get("id"),
            agent_id: r.get("agent_id"),
            status: r.get::<_, SqlTypeWrapper<SessionStatus>>("status").0,
            started_at: r.get("started_at"),
            ended_at: r.get("ended_at"),
        }))
    }

    /// Get sessions for an agent
    pub async fn get_agent_sessions(&self, agent_id: Uuid) -> crate::Result<Vec<AgentSession>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let rows = conn
            .query(
                r#"
                SELECT id, agent_id, status, started_at, ended_at
                FROM agent_sessions
                WHERE agent_id = $1
                ORDER BY started_at DESC
                "#,
                &[&agent_id],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| AgentSession {
                id: row.get("id"),
                agent_id: row.get("agent_id"),
                status: row.get::<_, SqlTypeWrapper<SessionStatus>>("status").0,
                started_at: row.get("started_at"),
                ended_at: row.get("ended_at"),
            })
            .collect())
    }

    /// Mark running sessions as exited on startup
    pub async fn mark_sessions_exited_on_startup(&self) -> crate::Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        conn.execute(
            r#"
            UPDATE agent_sessions
            SET status = 'exited', ended_at = NOW()
            WHERE status = 'running'
            "#,
            &[],
        )
        .await
        .map_err(|e| crate::TodokiError::Database(e))?;

        conn.execute(
            r#"
            UPDATE agents
            SET status = 'exited', updated_at = NOW()
            WHERE status = 'running'
            "#,
            &[],
        )
        .await
        .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    // ========================================================================
    // Artifact operations
    // ========================================================================

    /// Create a new artifact
    pub async fn create_artifact(
        &self,
        task_id: Uuid,
        project_id: Uuid,
        agent_id: Option<Uuid>,
        session_id: Option<Uuid>,
        artifact_type: &str,
        data: Value,
    ) -> crate::Result<Artifact> {
        let create = CreateArtifact::new(task_id, project_id, agent_id, session_id, artifact_type, data);

        let artifact_id = create
            .insert::<Artifact>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Artifact::fetch_one_by_pk(&artifact_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))
    }

    /// List artifacts for a project
    pub async fn list_artifacts(
        &self,
        project_id: Uuid,
        artifact_type: Option<&str>,
    ) -> crate::Result<Vec<Artifact>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let rows = if let Some(atype) = artifact_type {
            conn.query(
                r#"
                SELECT id, task_id, project_id, agent_id, session_id, artifact_type, data, created_at, updated_at
                FROM artifacts
                WHERE project_id = $1 AND artifact_type = $2
                ORDER BY created_at DESC
                "#,
                &[&project_id, &atype],
            )
            .await
        } else {
            conn.query(
                r#"
                SELECT id, task_id, project_id, agent_id, session_id, artifact_type, data, created_at, updated_at
                FROM artifacts
                WHERE project_id = $1
                ORDER BY created_at DESC
                "#,
                &[&project_id],
            )
            .await
        }
        .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Artifact {
                id: row.get("id"),
                task_id: row.get("task_id"),
                project_id: row.get("project_id"),
                agent_id: row.get("agent_id"),
                session_id: row.get("session_id"),
                artifact_type: row.get("artifact_type"),
                data: row.get("data"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    /// Get artifact by ID
    pub async fn get_artifact(&self, artifact_id: Uuid) -> crate::Result<Option<Artifact>> {
        match Artifact::fetch_one_by_pk(&artifact_id, &*self.pool).await {
            Ok(artifact) => Ok(Some(artifact)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::TodokiError::Database(e)),
        }
    }

    /// List artifacts by task ID
    pub async fn list_artifacts_by_task(&self, task_id: Uuid) -> crate::Result<Vec<Artifact>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let rows = conn
            .query(
                r#"
                SELECT id, task_id, project_id, agent_id, session_id, artifact_type, data, created_at, updated_at
                FROM artifacts
                WHERE task_id = $1
                ORDER BY created_at DESC
                "#,
                &[&task_id],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(rows
            .iter()
            .map(|row| Artifact {
                id: row.get("id"),
                task_id: row.get("task_id"),
                project_id: row.get("project_id"),
                agent_id: row.get("agent_id"),
                session_id: row.get("session_id"),
                artifact_type: row.get("artifact_type"),
                data: row.get("data"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

}
