//! Error handling for the API server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Authorization failed: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Nexus core error: {0}")]
    NexusCore(#[from] nexus_shared::NexusError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            ApiError::Auth(_) => (StatusCode::UNAUTHORIZED, "AUTH_FAILED", self.to_string()),
            ApiError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", self.to_string()),
            ApiError::NexusCore(_) => (StatusCode::BAD_GATEWAY, "NEXUS_ERROR", self.to_string()),
            ApiError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", self.to_string()),
            ApiError::Serialization(_) => (StatusCode::BAD_REQUEST, "SERIALIZATION_ERROR", self.to_string()),
            ApiError::HttpClient(_) => (StatusCode::BAD_GATEWAY, "HTTP_CLIENT_ERROR", self.to_string()),
            ApiError::Jwt(_) => (StatusCode::UNAUTHORIZED, "JWT_ERROR", self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", self.to_string()),
        };

        let body = json!({
            "error": {
                "code": error_code,
                "message": message,
                "timestamp": chrono::Utc::now(),
                "request_id": uuid::Uuid::new_v4()
            }
        });

        (status, Json(body)).into_response()
    }
}

// Conversion helpers
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}