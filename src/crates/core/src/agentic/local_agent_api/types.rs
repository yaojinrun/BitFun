use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
    pub workspace_path: String,
    pub message: String,
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalAgentTaskStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    NotFound,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionCandidate {
    pub session_id: String,
    pub session_name: String,
    pub agent_type: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunResponse {
    pub status: LocalAgentTaskStatus,
    pub session_id: String,
    pub session_name: String,
    pub turn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskQueryResponse {
    pub status: LocalAgentTaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    pub turn_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LocalAgentErrorCode {
    Unauthorized,
    InvalidRequest,
    SessionNotFound,
    SessionNameAmbiguous,
    SessionMismatch,
    SubmitFailed,
    TaskNotFound,
    InternalError,
}

impl LocalAgentErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionNameAmbiguous => "SESSION_NAME_AMBIGUOUS",
            Self::SessionMismatch => "SESSION_MISMATCH",
            Self::SubmitFailed => "SUBMIT_FAILED",
            Self::TaskNotFound => "TASK_NOT_FOUND",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAgentApiError {
    pub code: LocalAgentErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub details: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalAgentErrorResponse {
    pub error: LocalAgentApiError,
}

impl LocalAgentApiError {
    pub fn new(code: LocalAgentErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Map::new(),
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(LocalAgentErrorCode::InvalidRequest, message)
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }

    pub fn to_error_response(&self) -> LocalAgentErrorResponse {
        LocalAgentErrorResponse {
            error: self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_run_request_accepts_session_id_payload() {
        let request: TaskRunRequest = serde_json::from_value(json!({
            "sessionId": "session-1",
            "workspacePath": "D:\\BitFun",
            "message": "Run tests",
            "timeoutMs": 1000
        }))
        .expect("request should deserialize");

        assert_eq!(request.session_id.as_deref(), Some("session-1"));
        assert_eq!(request.session_name, None);
        assert_eq!(request.workspace_path, "D:\\BitFun");
        assert_eq!(request.message, "Run tests");
        assert_eq!(request.timeout_ms, Some(1000));
    }

    #[test]
    fn api_error_serializes_stable_code_and_message() {
        let error = LocalAgentApiError::invalid_request("message is required");
        let value = serde_json::to_value(error.to_error_response()).expect("serialize error");

        assert_eq!(value["error"]["code"], "INVALID_REQUEST");
        assert_eq!(value["error"]["message"], "message is required");
    }
}
