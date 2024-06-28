use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub error: String,
}

impl ErrorOutput {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("email already exists: {0}")]
    EmailAlreadyExists(String),
    #[error("create chat error: {0}")]
    CreateChatError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("permission deny")]
    PermissionDeny,
    #[error("sql error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("password hash error: {0}")]
    PasswordHashError(#[from] argon2::password_hash::Error),
    #[error("general error: {0}")]
    AnyError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::EmailAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::CreateChatError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::PermissionDeny => StatusCode::FORBIDDEN,
            AppError::SqlxError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::PasswordHashError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::AnyError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!(ErrorOutput::new(self.to_string())))).into_response()
    }
}
