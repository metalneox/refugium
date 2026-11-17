//! Errori dell'API, restituiti al client come JSON `{ "error": "..." }`.

use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("risorsa non trovata")]
    NotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Internal(error) => {
                tracing::error!("Errore interno: {error:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Errore interno del server".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        ApiError::Internal(error.into())
    }
}

impl From<MultipartError> for ApiError {
    fn from(error: MultipartError) -> Self {
        ApiError::BadRequest(format!("Upload non valido: {error}"))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        ApiError::Internal(error.into())
    }
}
