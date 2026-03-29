use axum::Router;

use crate::AppState;

mod api;

pub fn router() -> Router<AppState> {
    Router::new().nest("/api/v1", api::router())
}
