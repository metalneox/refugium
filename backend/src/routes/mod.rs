mod announcements;

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/announcements",
            get(announcements::list).post(announcements::create),
        )
        .route("/announcements/{id}", get(announcements::get_by_id))
}
