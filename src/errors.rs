use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// All error types that can occur within LabLink.
#[derive(Debug, Error)]
pub enum LinkError {
    /// Upstream server could not be reached (network / timeout).
    #[error("upstream unreachable: {0}")]
    UpstreamUnreachable(String),

    /// Upstream returned an unexpected HTTP status.
    #[error("upstream error {status}: {body}")]
    UpstreamStatus { status: u16, body: String },

    /// The response from the upstream could not be parsed.
    #[error("failed to parse upstream response: {0}")]
    ParseError(String),

    /// A generic internal error (wraps anyhow for convenience).
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<reqwest::Error> for LinkError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() || err.is_timeout() {
            LinkError::UpstreamUnreachable(err.to_string())
        } else if let Some(status) = err.status() {
            LinkError::UpstreamStatus {
                status: status.as_u16(),
                body: err.to_string(),
            }
        } else {
            LinkError::Internal(err.into())
        }
    }
}

/// Make `LinkError` directly returnable from axum handlers.
impl IntoResponse for LinkError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            LinkError::UpstreamUnreachable(_) => (
                StatusCode::BAD_GATEWAY,
                "upstream_unreachable",
                self.to_string(),
            ),
            LinkError::UpstreamStatus { .. } => {
                (StatusCode::BAD_GATEWAY, "upstream_error", self.to_string())
            }
            LinkError::ParseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "parse_error",
                self.to_string(),
            ),
            LinkError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                self.to_string(),
            ),
        };

        tracing::error!(error_code = code, %message);

        let body = Json(json!({ "error": code, "message": message }));
        (status, body).into_response()
    }
}

///Alias to use inside the codebase
pub type LinkResult<T> = Result<T, LinkError>;
