use crate::local_agent_api::auth::verify_authorization_header;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use bitfun_core::agentic::local_agent_api::{
    LocalAgentApiError, LocalAgentApiService, LocalAgentErrorCode, TaskRunRequest,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct LocalAgentHttpState {
    pub service: Arc<LocalAgentApiService>,
    pub token: Arc<String>,
}

pub fn router(state: LocalAgentHttpState) -> Router {
    Router::new()
        .route("/api/local-agent/tasks:run", post(run_task))
        .route("/api/local-agent/tasks/:turn_id", get(query_task))
        .with_state(state)
}

async fn run_task(
    State(state): State<LocalAgentHttpState>,
    headers: HeaderMap,
    Json(request): Json<TaskRunRequest>,
) -> Response {
    if let Err(error) = authorize(&headers, state.token.as_str()) {
        return error_response(error);
    }

    match state.service.run_task(request).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => error_response(error),
    }
}

async fn query_task(
    State(state): State<LocalAgentHttpState>,
    headers: HeaderMap,
    Path(turn_id): Path<String>,
) -> Response {
    if let Err(error) = authorize(&headers, state.token.as_str()) {
        return error_response(error);
    }

    (StatusCode::OK, Json(state.service.query_task(&turn_id))).into_response()
}

fn authorize(headers: &HeaderMap, expected_token: &str) -> Result<(), LocalAgentApiError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if verify_authorization_header(header, expected_token) {
        Ok(())
    } else {
        Err(LocalAgentApiError::new(
            LocalAgentErrorCode::Unauthorized,
            "missing or invalid bearer token",
        ))
    }
}

pub(crate) fn status_for_error(error: &LocalAgentApiError) -> StatusCode {
    match error.code {
        LocalAgentErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        LocalAgentErrorCode::InvalidRequest | LocalAgentErrorCode::SessionMismatch => {
            StatusCode::BAD_REQUEST
        }
        LocalAgentErrorCode::SessionNotFound | LocalAgentErrorCode::TaskNotFound => {
            StatusCode::NOT_FOUND
        }
        LocalAgentErrorCode::SessionNameAmbiguous => StatusCode::CONFLICT,
        LocalAgentErrorCode::SubmitFailed | LocalAgentErrorCode::InternalError => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn error_response(error: LocalAgentApiError) -> Response {
    let status = status_for_error(&error);
    (status, Json(error.to_error_response())).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitfun_core::agentic::local_agent_api::{
        LocalAgentApiError, LocalAgentErrorCode,
    };

    #[test]
    fn status_for_session_name_ambiguous_is_conflict() {
        let error = LocalAgentApiError::new(
            LocalAgentErrorCode::SessionNameAmbiguous,
            "multiple sessions match sessionName in this workspace",
        );
        assert_eq!(status_for_error(&error), axum::http::StatusCode::CONFLICT);
    }

    #[test]
    fn status_for_unauthorized_is_unauthorized() {
        let error = LocalAgentApiError::new(
            LocalAgentErrorCode::Unauthorized,
            "missing bearer token",
        );
        assert_eq!(status_for_error(&error), axum::http::StatusCode::UNAUTHORIZED);
    }
}
