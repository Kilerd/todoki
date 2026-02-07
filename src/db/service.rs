use crate::models::{
    agent::{
        Agent, AgentEvent, AgentSession, AgentStatus, CreateAgent, CreateAgentEvent,
        CreateAgentSession, SessionStatus,
    },
    report::{ReportPeriod, ReportResponse},
    task::{
        CreateTask, CreateTaskComment, CreateTaskEvent, Task, TaskComment, TaskEvent,
        TaskResponse, TaskStatus,
    },
};
use chrono::Utc;
use conservator::{Creatable, Domain, Executor, Migrator, PooledConnection};
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

        let status_values: Vec<String> = statuses.iter().map(|s| s.as_str().to_string()).collect();
        let placeholders: Vec<String> = (1..=status_values.len())
            .map(|i| format!("${}", i))
            .collect();

        let query = format!(
            r#"
            SELECT id, priority, content, "group", status, create_at, archived
            FROM tasks
            WHERE status IN ({})
              AND archived = false
            ORDER BY priority DESC, create_at DESC
            "#,
            placeholders.join(", ")
        );

        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            status_values.iter().map(|s| s as _).collect();

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
                group: row.get("group"),
                status: row.get("status"),
                create_at: row.get("create_at"),
                archived: row.get("archived"),
            })
            .collect())
    }

    /// Get today's tasks (todo, not archived)
    pub async fn get_today_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Todo]).await
    }

    /// Get inbox tasks (todo, in-progress, in-review, not archived)
    pub async fn get_inbox_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[
            TaskStatus::Todo,
            TaskStatus::InProgress,
            TaskStatus::InReview,
        ])
        .await
    }

    /// Get backlog tasks (backlog, not archived)
    pub async fn get_backlog_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Backlog]).await
    }

    /// Get in-progress tasks (in-progress or in-review, not archived)
    pub async fn get_in_progress_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::InProgress, TaskStatus::InReview])
            .await
    }

    /// Get done tasks (done, not archived)
    pub async fn get_done_tasks(&self) -> crate::Result<Vec<Task>> {
        self.get_tasks_by_status(&[TaskStatus::Done]).await
    }

    /// Get tasks marked done today (not archived)
    pub async fn get_today_done_tasks(&self) -> crate::Result<Vec<Task>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let query = r#"
            SELECT DISTINCT t.id, t.priority, t.content, t."group", t.status, t.create_at, t.archived
            FROM tasks t
            JOIN task_events e ON t.id = e.task_id
            WHERE t.status = 'done'
              AND t.archived = false
              AND e.event_type = 'StatusChange'
              AND e.state = 'done'
              AND (e.datetime AT TIME ZONE 'Asia/Hong_Kong')::date = (NOW() AT TIME ZONE 'Asia/Hong_Kong')::date
            ORDER BY t.priority DESC, t.create_at DESC
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
                group: row.get("group"),
                status: row.get("status"),
                create_at: row.get("create_at"),
                archived: row.get("archived"),
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

    /// Get full task response with events and comments
    pub async fn get_task_response(&self, task: Task) -> crate::Result<TaskResponse> {
        let events = self.get_task_events(task.id).await?;
        let comments = self.get_task_comments(task.id).await?;
        Ok(TaskResponse::from_task(task, events, comments))
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
        group: String,
    ) -> crate::Result<Task> {
        let mut task = Task::fetch_one_by_pk(&task_id, &*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        task.priority = priority;
        task.content = content;
        task.group = group;

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

        let old_status =
            TaskStatus::from_str(&task.status).unwrap_or_default();
        task.status = new_status.as_str().to_string();

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
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let row = conn
            .query_opt(
                r#"
                SELECT id, name, workdir, command, args, execution_mode, relay_id, status, created_at, updated_at
                FROM agents
                WHERE id = $1
                "#,
                &[&agent_id],
            )
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(row.map(|r| Agent {
            id: r.get("id"),
            name: r.get("name"),
            workdir: r.get("workdir"),
            command: r.get("command"),
            args: r.get("args"),
            execution_mode: r.get("execution_mode"),
            relay_id: r.get("relay_id"),
            status: r.get("status"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
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

        agent.status = status.as_str().to_string();
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
    pub async fn create_agent_session(
        &self,
        agent_id: Uuid,
        relay_id: Option<&str>,
    ) -> crate::Result<AgentSession> {
        let create = CreateAgentSession {
            agent_id,
            relay_id: relay_id.map(|s| s.to_string()),
        };

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

        session.status = status.as_str().to_string();
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
                SELECT id, agent_id, relay_id, status, started_at, ended_at
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
            relay_id: r.get("relay_id"),
            status: r.get("status"),
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
                SELECT id, agent_id, relay_id, status, started_at, ended_at
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
                relay_id: row.get("relay_id"),
                status: row.get("status"),
                started_at: row.get("started_at"),
                ended_at: row.get("ended_at"),
            })
            .collect())
    }

    // ========================================================================
    // Agent Event operations
    // ========================================================================

    /// Insert agent event
    pub async fn insert_agent_event(&self, event: CreateAgentEvent) -> crate::Result<()> {
        event
            .insert::<AgentEvent>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        Ok(())
    }

    /// Get agent events
    pub async fn get_agent_events(
        &self,
        agent_id: Uuid,
        limit: i64,
        before_seq: Option<i64>,
    ) -> crate::Result<Vec<AgentEvent>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| crate::TodokiError::Database(e))?;

        let rows = if let Some(before) = before_seq {
            conn.query(
                r#"
                SELECT id, agent_id, session_id, seq, ts, stream, message
                FROM agent_events
                WHERE agent_id = $1 AND seq < $2
                ORDER BY seq DESC
                LIMIT $3
                "#,
                &[&agent_id, &before, &limit],
            )
            .await
        } else {
            conn.query(
                r#"
                SELECT id, agent_id, session_id, seq, ts, stream, message
                FROM agent_events
                WHERE agent_id = $1
                ORDER BY seq DESC
                LIMIT $2
                "#,
                &[&agent_id, &limit],
            )
            .await
        }
        .map_err(|e| crate::TodokiError::Database(e))?;

        let mut events: Vec<AgentEvent> = rows
            .iter()
            .map(|row| AgentEvent {
                id: row.get("id"),
                agent_id: row.get("agent_id"),
                session_id: row.get("session_id"),
                seq: row.get("seq"),
                ts: row.get("ts"),
                stream: row.get("stream"),
                message: row.get("message"),
            })
            .collect();
        events.reverse();
        Ok(events)
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
}
