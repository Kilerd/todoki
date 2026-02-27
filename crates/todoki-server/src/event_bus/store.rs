use super::types::Event;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use conservator::{Executor, PooledConnection};
use std::sync::Arc;
use uuid::Uuid;

/// Event Store trait for persistence
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append a new event (returns assigned cursor)
    async fn append(&self, event: &mut Event) -> Result<i64>;

    /// Query events by cursor range
    async fn query(
        &self,
        from_cursor: i64,
        to_cursor: Option<i64>,
        kinds: Option<&[String]>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>>;

    /// Get latest cursor
    async fn latest_cursor(&self) -> Result<i64>;

    /// Delete events older than timestamp (for retention policy)
    async fn prune_before(&self, before: DateTime<Utc>) -> Result<u64>;
}

/// PostgreSQL implementation of Event Store
pub struct PgEventStore {
    pool: Arc<PooledConnection>,
}

impl PgEventStore {
    pub fn new(pool: Arc<PooledConnection>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStore for PgEventStore {
    async fn append(&self, event: &mut Event) -> Result<i64> {
        let conn = self.pool.get().await?;

        let row = conn
            .query_one(
                r#"
                INSERT INTO events (kind, time, agent_id, session_id, task_id, data)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING cursor
                "#,
                &[
                    &event.kind,
                    &event.time,
                    &event.agent_id,
                    &event.session_id,
                    &event.task_id,
                    &event.data,
                ],
            )
            .await?;

        let cursor: i64 = row.get("cursor");
        event.cursor = cursor;

        Ok(cursor)
    }

    async fn query(
        &self,
        from_cursor: i64,
        to_cursor: Option<i64>,
        kinds: Option<&[String]>,
        agent_id: Option<Uuid>,
        task_id: Option<Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        let conn = self.pool.get().await?;
        let limit_i64 = limit.unwrap_or(1000).min(10000) as i64;

        // Convert Option<&[String]> to Option<Vec<String>> for query parameter
        let kinds_vec = kinds.map(|k| k.to_vec());

        let rows = conn
            .query(
                r#"
                SELECT cursor, kind, time, agent_id, session_id, task_id, data
                FROM events
                WHERE cursor > $1
                  AND ($2::BIGINT IS NULL OR cursor <= $2)
                  AND ($3::TEXT[] IS NULL OR kind = ANY($3))
                  AND ($4::UUID IS NULL OR agent_id = $4)
                  AND ($5::UUID IS NULL OR task_id = $5)
                ORDER BY cursor ASC
                LIMIT $6
                "#,
                &[
                    &from_cursor,
                    &to_cursor,
                    &kinds_vec,
                    &agent_id,
                    &task_id,
                    &limit_i64,
                ],
            )
            .await?;

        let events = rows
            .iter()
            .map(|row| Event {
                cursor: row.get("cursor"),
                kind: row.get("kind"),
                time: row.get("time"),
                agent_id: row.get("agent_id"),
                session_id: row.get("session_id"),
                task_id: row.get("task_id"),
                data: row.get("data"),
            })
            .collect();

        Ok(events)
    }

    async fn latest_cursor(&self) -> Result<i64> {
        let conn = self.pool.get().await?;

        let row = conn
            .query_one("SELECT COALESCE(MAX(cursor), 0) as cursor FROM events", &[])
            .await?;

        let cursor: i64 = row.get("cursor");
        Ok(cursor)
    }

    async fn prune_before(&self, before: DateTime<Utc>) -> Result<u64> {
        let conn = self.pool.get().await?;

        let result = conn
            .execute("DELETE FROM events WHERE time < $1", &[&before])
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_store_trait() {
        // Compile-time check that PgEventStore implements EventStore
        fn _assert_impl<T: EventStore>() {}
        _assert_impl::<PgEventStore>();
    }
}
