use crate::models::{
    report::{ReportPeriod, ReportResponse},
    task::{
        CreateTask, CreateTaskComment, CreateTaskEvent, Task, TaskComment, TaskEvent,
        TaskResponse, TaskStatus,
    },
};
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
}
