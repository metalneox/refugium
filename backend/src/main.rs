//! Backend di Refugium: API per gli annunci di cani, costruita con Axum.

mod auth;
mod db;
mod error;
mod routes;

use std::path::PathBuf;

use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use axum::{Json, Router};
use sqlx::sqlite::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::error::ApiError;

/// Stato condiviso con tutti gli handler.
#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    uploads_dir: PathBuf,
    auth: auth::Auth,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("backend=debug,tower_http=debug")),
        )
        .init();

    // Configurazione via variabili d'ambiente (con valori predefiniti sensati).
    let db_url = std::env::var("REFUGIUM_DB_URL")
        .unwrap_or_else(|_| "sqlite://data/refugium.db?mode=rwc".to_string());
    let uploads_dir = PathBuf::from(
        std::env::var("REFUGIUM_UPLOADS_DIR").unwrap_or_else(|_| "data/uploads".to_string()),
    );

    // SQLite non crea le cartelle mancanti: preparale prima di connetterti.
    tokio::fs::create_dir_all(&uploads_dir).await?;
    let db_file = db_url
        .strip_prefix("sqlite://")
        .unwrap_or(&db_url)
        .split('?')
        .next()
        .unwrap_or_default()
        .to_string();
    if let Some(parent) = std::path::Path::new(&db_file).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let pool = SqlitePool::connect(&db_url).await?;
    db::ensure_schema(&pool).await?;

    let state = AppState {
        pool,
        uploads_dir,
        auth: auth::Auth::from_env(),
    };

    let app = Router::new()
        .nest("/api", routes::router())
        .route("/health", get(health))
        // Le immagini caricate sono servite come file statici.
        .nest_service("/uploads", ServeDir::new(&state.uploads_dir))
        // In produzione serve il frontend compilato con `trunk build`
        // (se la cartella `frontend/dist` esiste).
        .fallback_service(ServeDir::new("frontend/dist"))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        // Sviluppo: il frontend gira su un'origine diversa (dev server Trunk).
        .layer(CorsLayer::permissive())
        // ~12 MB totali per richiesta (immagine + campi del form).
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024));

    let addr = std::env::var("REFUGIUM_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Backend in ascolto su http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

/// `GET /health` — probe di base.
async fn health() -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({ "status": "ok" })))
}
