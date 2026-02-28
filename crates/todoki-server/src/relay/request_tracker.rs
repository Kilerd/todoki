use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Tracks pending requests and their response channels
///
/// Used to implement async request-response pattern over Event Bus:
/// 1. Server emits command event with unique request_id
/// 2. Calls track_request() to get a receiver
/// 3. Waits on the receiver (with timeout)
/// 4. Background handler listens to response events
/// 5. Calls complete_request() to send response through channel
pub struct RequestTracker {
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Track a new request and return a receiver for the response
    ///
    /// The caller should wait on the receiver (typically with a timeout).
    /// When the response arrives, it will be sent through this channel.
    pub async fn track_request(&self, request_id: String) -> oneshot::Receiver<Result<Value>> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, tx);
        rx
    }

    /// Complete a tracked request by sending the response
    ///
    /// This is called by the background response handler when a response event arrives.
    /// If the request_id is not found (e.g., already completed or timed out), this is a no-op.
    pub async fn complete_request(&self, request_id: &str, result: Result<Value>) {
        if let Some(tx) = self.pending.lock().await.remove(request_id) {
            // Ignore send errors (receiver may have been dropped due to timeout)
            let _ = tx.send(result);
        }
    }

    /// Cancel a tracked request
    ///
    /// This is useful when the caller wants to explicitly cancel waiting for a response.
    /// In practice, timeout handles most cases, so this is rarely needed.
    pub async fn cancel_request(&self, request_id: &str) {
        self.pending.lock().await.remove(request_id);
    }

    /// Get the number of pending requests (for monitoring)
    #[cfg(test)]
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_response_flow() {
        let tracker = RequestTracker::new();

        // Track a request
        let request_id = "test-123".to_string();
        let rx = tracker.track_request(request_id.clone()).await;

        assert_eq!(tracker.pending_count().await, 1);

        // Complete the request
        let response = serde_json::json!({"status": "ok"});
        tracker
            .complete_request(&request_id, Ok(response.clone()))
            .await;

        assert_eq!(tracker.pending_count().await, 0);

        // Receive the response
        let result = rx.await.unwrap().unwrap();
        assert_eq!(result, response);
    }

    #[tokio::test]
    async fn test_cancel_request() {
        let tracker = RequestTracker::new();

        let request_id = "test-456".to_string();
        let _rx = tracker.track_request(request_id.clone()).await;

        assert_eq!(tracker.pending_count().await, 1);

        tracker.cancel_request(&request_id).await;

        assert_eq!(tracker.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_complete_unknown_request() {
        let tracker = RequestTracker::new();

        // Completing an unknown request should not panic
        tracker
            .complete_request("unknown", Ok(serde_json::json!({})))
            .await;

        assert_eq!(tracker.pending_count().await, 0);
    }
}
