use serde::{Deserialize, Serialize};
use std::fmt;

/// Request to start a local agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionRequest {
    pub agent_id: String,
    pub config: Option<serde_json::Value>,
}

/// Response when starting a local agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionResponse {
    pub session_id: String,
    pub status: String,
}

/// Request to send a message to a local agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub session_id: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}

/// Response when sending a message to a local agent session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SendMessageResponse {
    pub session_id: String,
    pub message_id: String,
    pub status: String,
}

/// Request to stop a local agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSessionRequest {
    pub session_id: String,
}

/// Response when stopping a local agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopSessionResponse {
    pub session_id: String,
    pub status: String,
}

/// Error types for the local agent API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalAgentApiError {
    InvalidRequest(String),
    SessionNotFound(String),
    AgentNotFound(String),
    InternalError(String),
    Timeout,
}

impl fmt::Display for LocalAgentApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalAgentApiError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LocalAgentApiError::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
            LocalAgentApiError::AgentNotFound(msg) => write!(f, "Agent not found: {}", msg),
            LocalAgentApiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            LocalAgentApiError::Timeout => write!(f, "Request timeout"),
        }
    }
}

impl std::error::Error for LocalAgentApiError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_start_session_request_serialization() {
        let request = StartSessionRequest {
            agent_id: "test-agent".to_string(),
            config: Some(json!({"key": "value"})),
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: StartSessionRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.agent_id, "test-agent");
        assert_eq!(deserialized.config, Some(json!({"key": "value"})));
    }

    #[test]
    fn test_start_session_response_serialization() {
        let response = StartSessionResponse {
            session_id: "session-123".to_string(),
            status: "started".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: StartSessionResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "session-123");
        assert_eq!(deserialized.status, "started");
    }

    #[test]
    fn test_send_message_request_serialization() {
        let request = SendMessageRequest {
            session_id: "session-123".to_string(),
            message: "Hello, agent!".to_string(),
            metadata: Some(json!({"priority": "high"})),
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: SendMessageRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "session-123");
        assert_eq!(deserialized.message, "Hello, agent!");
        assert_eq!(deserialized.metadata, Some(json!({"priority": "high"})));
    }

    #[test]
    fn test_send_message_response_serialization() {
        let response = SendMessageResponse {
            session_id: "session-123".to_string(),
            message_id: "msg-456".to_string(),
            status: "sent".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: SendMessageResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "session-123");
        assert_eq!(deserialized.message_id, "msg-456");
        assert_eq!(deserialized.status, "sent");
    }

    #[test]
    fn test_stop_session_request_serialization() {
        let request = StopSessionRequest {
            session_id: "session-123".to_string(),
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: StopSessionRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "session-123");
    }

    #[test]
    fn test_stop_session_response_serialization() {
        let response = StopSessionResponse {
            session_id: "session-123".to_string(),
            status: "stopped".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: StopSessionResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_id, "session-123");
        assert_eq!(deserialized.status, "stopped");
    }

    #[test]
    fn test_local_agent_api_error_display() {
        let error = LocalAgentApiError::InvalidRequest("test error".to_string());
        assert_eq!(error.to_string(), "Invalid request: test error");
        
        let error = LocalAgentApiError::SessionNotFound("session not found".to_string());
        assert_eq!(error.to_string(), "Session not found: session not found");
        
        let error = LocalAgentApiError::AgentNotFound("agent not found".to_string());
        assert_eq!(error.to_string(), "Agent not found: agent not found");
        
        let error = LocalAgentApiError::InternalError("internal error".to_string());
        assert_eq!(error.to_string(), "Internal error: internal error");
        
        let error = LocalAgentApiError::Timeout;
        assert_eq!(error.to_string(), "Request timeout");
    }
}