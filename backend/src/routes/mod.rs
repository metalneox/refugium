mod announcements;
mod auth;

use axum::Router;
use axum::routing::{get, post};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/announcements",
            get(announcements::list).post(announcements::create),
        )
        .route("/announcements/{id}", get(announcements::get_by_id))
        // Registrazione e login sono pubblici; `/me` richiede il Bearer token.
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/me", get(auth::me))
}
