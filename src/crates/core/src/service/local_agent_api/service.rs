use super::tracker::{TaskRegistration, TaskResultTracker};
use super::types::{
    LocalAgentApiError, LocalAgentErrorCode, LocalAgentTaskStatus, SessionCandidate,
    TaskQueryResponse, TaskRunRequest, TaskRunResponse,
};
use crate::agentic::coordination::{
    ConversationCoordinator, DialogScheduler, DialogSubmissionPolicy, DialogTriggerSource,
};
use crate::agentic::core::SessionSummary;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DEFAULT_TIMEOUT_MS: u64 = 600_000;
const MAX_TIMEOUT_MS: u64 = 3_600_000;

#[derive(Clone)]
pub struct LocalAgentApiService {
    coordinator: Arc<ConversationCoordinator>,
    scheduler: Arc<DialogScheduler>,
    tracker: Arc<TaskResultTracker>,
}

#[derive(Debug, Clone)]
struct ResolvedSession {
    session_id: String,
    session_name: String,
    agent_type: String,
}

impl LocalAgentApiService {
    pub fn new(
        coordinator: Arc<ConversationCoordinator>,
        scheduler: Arc<DialogScheduler>,
        tracker: Arc<TaskResultTracker>,
    ) -> Self {
        Self {
            coordinator,
            scheduler,
            tracker,
        }
    }

    pub async fn run_task(
        &self,
        request: TaskRunRequest,
    ) -> Result<TaskRunResponse, LocalAgentApiError> {
        validate_task_request(&request)?;
        let workspace_path = PathBuf::from(request.workspace_path.trim());
        let session = self.resolve_session(&workspace_path, &request).await?;
        let turn_id = format!("local-agent-{}", Uuid::new_v4());
        let agent_type = request
            .agent_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&session.agent_type)
            .to_string();

        self.tracker.register(TaskRegistration {
            turn_id: turn_id.clone(),
            session_id: session.session_id.clone(),
            session_name: session.session_name.clone(),
        });

        self.scheduler
            .submit(
                session.session_id.clone(),
                request.message.clone(),
                Some(request.message.clone()),
                Some(turn_id.clone()),
                agent_type,
                Some(request.workspace_path.clone()),
                DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi),
                None,
                None,
                None,
            )
            .await
            .map_err(|error| {
                LocalAgentApiError::new(LocalAgentErrorCode::SubmitFailed, error)
            })?;

        let timeout = Duration::from_millis(resolve_timeout_ms(request.timeout_ms));
        let waited = self.tracker.wait_for(&turn_id, timeout).await;
        let query = waited.unwrap_or_else(|| self.tracker.query_or_not_found(&turn_id));
        Ok(task_run_response_from_query(query, true))
    }

    pub fn query_task(&self, turn_id: &str) -> TaskQueryResponse {
        self.tracker.query_or_not_found(turn_id)
    }

    async fn resolve_session(
        &self,
        workspace_path: &PathBuf,
        request: &TaskRunRequest,
    ) -> Result<ResolvedSession, LocalAgentApiError> {
        let sessions = self.coordinator.list_sessions(workspace_path).await.map_err(|error| {
            LocalAgentApiError::new(LocalAgentErrorCode::InternalError, error.to_string())
        })?;

        resolve_session_from_summaries(&sessions, request)
    }
}

fn resolve_timeout_ms(timeout_ms: Option<u64>) -> u64 {
    timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(1, MAX_TIMEOUT_MS)
}

fn system_time_to_unix_secs(value: SystemTime) -> u64 {
    value
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn candidate_from_summary(summary: &SessionSummary) -> SessionCandidate {
    SessionCandidate {
        session_id: summary.session_id.clone(),
        session_name: summary.session_name.clone(),
        agent_type: summary.agent_type.clone(),
        created_at: system_time_to_unix_secs(summary.created_at),
    }
}

pub(crate) fn validate_task_request(request: &TaskRunRequest) -> Result<(), LocalAgentApiError> {
    let has_session_id = request
        .session_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let has_session_name = request
        .session_name
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());

    if !has_session_id && !has_session_name {
        return Err(LocalAgentApiError::invalid_request(
            "sessionId or sessionName is required",
        ));
    }
    if request.workspace_path.trim().is_empty() {
        return Err(LocalAgentApiError::invalid_request("workspacePath is required"));
    }
    if request.message.trim().is_empty() {
        return Err(LocalAgentApiError::invalid_request("message is required"));
    }

    Ok(())
}

fn resolve_session_from_summaries(
    sessions: &[SessionSummary],
    request: &TaskRunRequest,
) -> Result<ResolvedSession, LocalAgentApiError> {
    let by_id = request.session_id.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let by_name = request.session_name.as_deref().map(str::trim).filter(|value| !value.is_empty());

    if let Some(session_id) = by_id {
        let session = sessions
            .iter()
            .find(|summary| summary.session_id == session_id)
            .ok_or_else(|| {
                LocalAgentApiError::new(
                    LocalAgentErrorCode::SessionNotFound,
                    format!("sessionId '{}' was not found", session_id),
                )
            })?;

        if let Some(session_name) = by_name {
            if session.session_name != session_name {
                return Err(LocalAgentApiError::new(
                    LocalAgentErrorCode::SessionMismatch,
                    "sessionId and sessionName do not refer to the same session",
                ));
            }
        }

        return Ok(ResolvedSession {
            session_id: session.session_id.clone(),
            session_name: session.session_name.clone(),
            agent_type: session.agent_type.clone(),
        });
    }

    let session_name = by_name.expect("validated sessionName exists");
    let matches: Vec<&SessionSummary> = sessions
        .iter()
        .filter(|summary| summary.session_name == session_name)
        .collect();

    match matches.as_slice() {
        [] => Err(LocalAgentApiError::new(
            LocalAgentErrorCode::SessionNotFound,
            format!("sessionName '{}' was not found", session_name),
        )),
        [session] => Ok(ResolvedSession {
            session_id: session.session_id.clone(),
            session_name: session.session_name.clone(),
            agent_type: session.agent_type.clone(),
        }),
        _ => {
            let candidates: Vec<SessionCandidate> =
                matches.into_iter().map(candidate_from_summary).collect();
            Err(LocalAgentApiError::new(
                LocalAgentErrorCode::SessionNameAmbiguous,
                "multiple sessions match sessionName in this workspace",
            )
            .with_detail("candidates", json!(candidates)))
        }
    }
}

fn task_run_response_from_query(query: TaskQueryResponse, include_timeout: bool) -> TaskRunResponse {
    let timed_out = include_timeout && query.status == LocalAgentTaskStatus::Running;
    TaskRunResponse {
        status: query.status,
        session_id: query.session_id.unwrap_or_default(),
        session_name: query.session_name.unwrap_or_default(),
        turn_id: query.turn_id,
        final_response: query.final_response,
        error: query.error,
        timed_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(session_id: Option<&str>, session_name: Option<&str>) -> TaskRunRequest {
        TaskRunRequest {
            session_id: session_id.map(str::to_string),
            session_name: session_name.map(str::to_string),
            workspace_path: "D:\\BitFun".to_string(),
            message: "Do work".to_string(),
            agent_type: None,
            timeout_ms: Some(1000),
        }
    }

    #[test]
    fn validate_rejects_missing_session_identifier() {
        let error = validate_task_request(&request(None, None)).expect_err("must fail");
        assert_eq!(error.code, LocalAgentErrorCode::InvalidRequest);
        assert_eq!(
            error.message,
            "sessionId or sessionName is required"
        );
    }

    #[test]
    fn validate_rejects_empty_message() {
        let mut req = request(Some("session-1"), None);
        req.message = "   ".to_string();
        let error = validate_task_request(&req).expect_err("must fail");
        assert_eq!(error.message, "message is required");
    }

    #[test]
    fn validate_accepts_session_name_request() {
        validate_task_request(&request(None, Some("Worker"))).expect("valid request");
    }
}
