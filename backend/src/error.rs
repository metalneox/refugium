//! Errori dell'API, restituiti al client come JSON `{ "error": "..." }`.

use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Conflict(String),
    #[error("risorsa non trovata")]
    NotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            ApiError::Conflict(message) => (StatusCode::CONFLICT, message),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Risorsa non trovata".to_string()),
            ApiError::Internal(error) => {
                tracing::error!("Errore interno: {error:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Errore interno del server".to_string(),
                )
            }
        };

        let mut response = (status, Json(json!({ "error": message }))).into_response();
        if status == StatusCode::UNAUTHORIZED {
            // Indica al client che è richiesta l'autenticazione Bearer.
            response
                .headers_mut()
                .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
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
