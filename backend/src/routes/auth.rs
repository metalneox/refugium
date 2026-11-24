//! Endpoint di registrazione, login e sessione corrente.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use shared::{AuthResponse, AuthUser, Credentials};

use crate::AppState;
use crate::auth::{Auth, AuthUser as Authenticated, validate_credentials};
use crate::db::UserRow;
use crate::error::ApiError;

/// `POST /api/auth/register` — crea un account e restituisce subito un token.
pub async fn register(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    validate_credentials(&credentials)?;

    let email = credentials.email.trim().to_lowercase();
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = Auth::hash_password(&credentials.password)?;
    let created_at = Utc::now().to_rfc3339();

    // `INSERT OR IGNORE` gestisce anche la corsa tra due registrazioni concorrenti.
    let result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO users (id, email, password_hash, created_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
    )
    .bind(&id)
    .bind(&email)
    .bind(&password_hash)
    .bind(&created_at)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::Conflict("Email già registrata".into()));
    }

    let token = state.auth.issue_token(&id, &email)?;
    tracing::info!("Nuovo utente registrato: {email}");

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: AuthUser {
                id,
                email,
                created_at,
            },
        }),
    ))
}

/// `POST /api/auth/login` — verifica le credenziali e restituisce un token.
pub async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<AuthResponse>, ApiError> {
    let email = credentials.email.trim().to_lowercase();
    let invalid = || ApiError::Unauthorized("Email o password non corretti".into());

    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(invalid)?;

    if !Auth::verify_password(&credentials.password, &row.password_hash) {
        return Err(invalid());
    }

    let token = state.auth.issue_token(&row.id, &row.email)?;

    Ok(Json(AuthResponse {
        token,
        user: AuthUser {
            id: row.id,
            email: row.email,
            created_at: row.created_at,
        },
    }))
}

/// `GET /api/auth/me` — dati dell'utente del token (usato per ripristinare la sessione).
pub async fn me(
    State(state): State<AppState>,
    authenticated: Authenticated,
) -> Result<Json<AuthUser>, ApiError> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?1")
        .bind(&authenticated.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Utente non trovato".into()))?;

    Ok(Json(AuthUser {
        id: row.id,
        email: row.email,
        created_at: row.created_at,
    }))
}
