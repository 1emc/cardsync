use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("unauthorized")]
    UnauthorizedBasic,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // The WWW-Authenticate challenge is required so that CardDAV clients
        // (e.g. iOS Contacts) actually prompt for credentials on a 401.
        let (status, message, challenge) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                Some(HeaderValue::from_static("Bearer")),
            ),
            Self::UnauthorizedBasic => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                Some(HeaderValue::from_static("Basic realm=\"galcard CardDAV\"")),
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string(), None),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_string(), None),
            // Validation messages are safe to surface and help admins debug bad requests.
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, None),
            // Internal errors stay generic so we never leak SQL or upstream details.
            Self::Sqlx(_) | Self::Anyhow(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
                None,
            ),
        };
        let mut response = (status, Json(json!({ "error": message }))).into_response();
        if let Some(challenge) = challenge {
            response.headers_mut().insert(WWW_AUTHENTICATE, challenge);
        }
        response
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
