use crate::{config::PontemeshHome, setup, web_assets};
use axum::{
    Router,
    routing::{any, get, post},
};
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub paths: PontemeshHome,
    pub setup: setup::SetupState,
}

pub fn router(paths: PontemeshHome, setup: setup::SetupState) -> Router {
    let state = AppState { paths, setup };

    Router::new()
        .route("/api/setup/status", get(setup::routes::status))
        .route("/api/setup/unlock", post(setup::routes::unlock))
        .route("/api/setup/complete", post(setup::routes::complete))
        .route("/api/{*path}", any(setup::routes::api_not_found))
        .fallback(web_assets::serve)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
