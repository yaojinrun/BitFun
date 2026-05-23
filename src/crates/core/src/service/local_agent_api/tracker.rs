use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::service::local_agent_api::types::{SendMessageResponse, LocalAgentApiError};

/// Tracks results of local agent API tasks
#[derive(Debug, Clone)]
pub struct TaskResultTracker {
    /// Map of task IDs to their results
    results: Arc<Mutex<HashMap<String, SendMessageResponse>>>,
}

impl TaskResultTracker {
    /// Creates a new task result tracker
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Stores a task result
    pub fn store_result(&self, task_id: String, result: SendMessageResponse) {
        let mut results = self.results.lock().unwrap();
        results.insert(task_id, result);
    }

    /// Retrieves a task result by ID
    pub fn get_result(&self, task_id: &str) -> Result<SendMessageResponse, LocalAgentApiError> {
        let results = self.results.lock().unwrap();
        results.get(task_id)
            .cloned()
            .ok_or_else(|| LocalAgentApiError::SessionNotFound(format!("Task result not found for ID: {}", task_id)))
    }

    /// Removes a task result by ID
    pub fn remove_result(&self, task_id: &str) -> Option<SendMessageResponse> {
        let mut results = self.results.lock().unwrap();
        results.remove(task_id)
    }

    /// Clears all task results
    pub fn clear(&self) {
        let mut results = self.results.lock().unwrap();
        results.clear();
    }

    /// Gets the number of tracked task results
    pub fn len(&self) -> usize {
        let results = self.results.lock().unwrap();
        results.len()
    }

    /// Checks if no task results are being tracked
    pub fn is_empty(&self) -> bool {
        let results = self.results.lock().unwrap();
        results.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::local_agent_api::types::{SendMessageResponse, LocalAgentApiError};

    #[test]
    fn test_tracker_new() {
        let tracker = TaskResultTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_tracker_store_and_get_result() {
        let tracker = TaskResultTracker::new();
        let task_id = "task-123".to_string();
        let result = SendMessageResponse {
            session_id: "session-456".to_string(),
            message_id: "msg-789".to_string(),
            status: "completed".to_string(),
        };

        // Store result
        tracker.store_result(task_id.clone(), result.clone());

        // Retrieve result
        let retrieved = tracker.get_result(&task_id).unwrap();
        assert_eq!(retrieved, result);
    }

    #[test]
    fn test_tracker_get_nonexistent_result() {
        let tracker = TaskResultTracker::new();
        let task_id = "nonexistent";

        // Try to get a result that doesn't exist
        let error = tracker.get_result(task_id).unwrap_err();
        assert!(matches!(error, LocalAgentApiError::SessionNotFound(_)));
    }

    #[test]
    fn test_tracker_remove_result() {
        let tracker = TaskResultTracker::new();
        let task_id = "task-123".to_string();
        let result = SendMessageResponse {
            session_id: "session-456".to_string(),
            message_id: "msg-789".to_string(),
            status: "completed".to_string(),
        };

        // Store result
        tracker.store_result(task_id.clone(), result.clone());

        // Remove result
        let removed = tracker.remove_result(&task_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap(), result);

        // Try to get removed result
        let error = tracker.get_result(&task_id).unwrap_err();
        assert!(matches!(error, LocalAgentApiError::SessionNotFound(_)));

        // Try to remove again (should return None)
        let removed_again = tracker.remove_result(&task_id);
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_tracker_clear() {
        let tracker = TaskResultTracker::new();
        let task_id1 = "task-123".to_string();
        let task_id2 = "task-456".to_string();
        let result1 = SendMessageResponse {
            session_id: "session-456".to_string(),
            message_id: "msg-789".to_string(),
            status: "completed".to_string(),
        };
        let result2 = SendMessageResponse {
            session_id: "session-789".to_string(),
            message_id: "msg-012".to_string(),
            status: "failed".to_string(),
        };

        // Store two results
        tracker.store_result(task_id1.clone(), result1.clone());
        tracker.store_result(task_id2.clone(), result2.clone());

        assert_eq!(tracker.len(), 2);
        assert!(!tracker.is_empty());

        // Clear tracker
        tracker.clear();

        assert_eq!(tracker.len(), 0);
        assert!(tracker.is_empty());

        // Verify results are gone
        let error1 = tracker.get_result(&task_id1).unwrap_err();
        let error2 = tracker.get_result(&task_id2).unwrap_err();
        assert!(matches!(error1, LocalAgentApiError::SessionNotFound(_)));
        assert!(matches!(error2, LocalAgentApiError::SessionNotFound(_)));
    }

    #[test]
    fn test_tracker_clone() {
        let tracker1 = TaskResultTracker::new();
        let task_id = "task-123".to_string();
        let result = SendMessageResponse {
            session_id: "session-456".to_string(),
            message_id: "msg-789".to_string(),
            status: "completed".to_string(),
        };

        // Store result in original tracker
        tracker1.store_result(task_id.clone(), result.clone());

        // Clone tracker
        let tracker2 = tracker1.clone();

        // Both should have the result
        assert_eq!(tracker1.get_result(&task_id).unwrap(), result);
        assert_eq!(tracker2.get_result(&task_id).unwrap(), result);
    }
}