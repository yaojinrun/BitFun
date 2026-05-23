use super::types::{LocalAgentTaskStatus, TaskQueryResponse};
use crate::agentic::coordination::turn_outcome::TurnOutcome;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct TaskRegistration {
    pub turn_id: String,
    pub session_id: String,
    pub session_name: String,
}

#[derive(Debug, Clone)]
struct TrackedTask {
    response: TaskQueryResponse,
    created_at: SystemTime,
    notify: Arc<Notify>,
}

#[derive(Debug, Default)]
pub struct TaskResultTracker {
    tasks: DashMap<String, TrackedTask>,
}

impl TaskResultTracker {
    pub fn register(&self, registration: TaskRegistration) {
        self.tasks.insert(
            registration.turn_id.clone(),
            TrackedTask {
                response: TaskQueryResponse {
                    status: LocalAgentTaskStatus::Running,
                    session_id: Some(registration.session_id),
                    session_name: Some(registration.session_name),
                    turn_id: registration.turn_id,
                    final_response: None,
                    error: None,
                },
                created_at: SystemTime::now(),
                notify: Arc::new(Notify::new()),
            },
        );
    }

    pub fn query(&self, turn_id: &str) -> Option<TaskQueryResponse> {
        self.tasks.get(turn_id).map(|entry| entry.response.clone())
    }

    pub fn query_or_not_found(&self, turn_id: &str) -> TaskQueryResponse {
        self.query(turn_id).unwrap_or_else(|| TaskQueryResponse {
            status: LocalAgentTaskStatus::NotFound,
            session_id: None,
            session_name: None,
            turn_id: turn_id.to_string(),
            final_response: None,
            error: None,
        })
    }

    pub async fn wait_for(&self, turn_id: &str, timeout: Duration) -> Option<TaskQueryResponse> {
        if let Some(existing) = self.query(turn_id) {
            if existing.status != LocalAgentTaskStatus::Running {
                return Some(existing);
            }
        }

        let notify = self.tasks.get(turn_id).map(|entry| entry.notify.clone())?;
        let notified = notify.notified();
        tokio::select! {
            _ = notified => self.query(turn_id),
            _ = tokio::time::sleep(timeout) => None,
        }
    }

    pub fn record_outcome(&self, session_id: &str, outcome: TurnOutcome) {
        let turn_id = outcome.turn_id().to_string();
        let Some(mut entry) = self.tasks.get_mut(&turn_id) else {
            return;
        };

        if entry.response.session_id.as_deref() != Some(session_id) {
            return;
        }

        match outcome {
            TurnOutcome::Completed { final_response, .. } => {
                entry.response.status = LocalAgentTaskStatus::Completed;
                entry.response.final_response = Some(final_response);
                entry.response.error = None;
            }
            TurnOutcome::Cancelled { .. } => {
                entry.response.status = LocalAgentTaskStatus::Cancelled;
                entry.response.final_response = None;
                entry.response.error = None;
            }
            TurnOutcome::Failed { error, .. } => {
                entry.response.status = LocalAgentTaskStatus::Failed;
                entry.response.final_response = None;
                entry.response.error = Some(error);
            }
        }

        entry.notify.notify_waiters();
    }

    pub fn prune_older_than(&self, max_age: Duration) {
        let now = SystemTime::now();
        let expired: Vec<String> = self
            .tasks
            .iter()
            .filter_map(|entry| {
                now.duration_since(entry.created_at)
                    .ok()
                    .filter(|age| *age > max_age)
                    .map(|_| entry.key().clone())
            })
            .collect();

        for turn_id in expired {
            self.tasks.remove(&turn_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::coordination::turn_outcome::TurnOutcome;

    #[tokio::test]
    async fn registered_task_is_running_until_completed() {
        let tracker = TaskResultTracker::default();
        tracker.register(TaskRegistration {
            turn_id: "turn-1".to_string(),
            session_id: "session-1".to_string(),
            session_name: "Worker".to_string(),
        });

        let before = tracker.query("turn-1").expect("task should exist");
        assert_eq!(before.status, LocalAgentTaskStatus::Running);

        tracker.record_outcome(
            "session-1",
            TurnOutcome::Completed {
                turn_id: "turn-1".to_string(),
                final_response: "done".to_string(),
            },
        );

        let after = tracker.query("turn-1").expect("task should exist");
        assert_eq!(after.status, LocalAgentTaskStatus::Completed);
        assert_eq!(after.final_response.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn wait_returns_none_when_timeout_expires() {
        let tracker = TaskResultTracker::default();
        tracker.register(TaskRegistration {
            turn_id: "turn-2".to_string(),
            session_id: "session-2".to_string(),
            session_name: "Worker".to_string(),
        });

        let result = tracker
            .wait_for("turn-2", std::time::Duration::from_millis(1))
            .await;

        assert!(result.is_none());
    }
}
