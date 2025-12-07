use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::multipart::MultipartError;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{status}: {message}")]
    Http { status: StatusCode, message: String },
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Http { status, message } => (*status, message.clone()),
            AppError::Sqlx(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database failure".to_string(),
            ),
            AppError::Io(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage failure".to_string(),
            ),
            AppError::Serde(_) | AppError::Toml(_) => {
                (StatusCode::BAD_REQUEST, "serialization error".to_string())
            }
        };
        let body = Json(ErrorBody { error: message });
        (status, body).into_response()
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::UNAUTHORIZED,
            message: msg.into(),
        }
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::CONFLICT,
            message: msg.into(),
        }
    }

    pub fn payload_too_large(msg: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: msg.into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::Http {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::bad_request(err.to_string())
    }
}

impl From<MultipartError> for AppError {
    fn from(err: MultipartError) -> Self {
        match err.status() {
            StatusCode::PAYLOAD_TOO_LARGE => {
                AppError::payload_too_large("multipart payload too large")
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                AppError::unauthorized(err.to_string())
            }
            StatusCode::CONFLICT => AppError::conflict(err.to_string()),
            StatusCode::NOT_FOUND => AppError::not_found(err.to_string()),
            _ => AppError::bad_request(err.to_string()),
        }
    }
}
