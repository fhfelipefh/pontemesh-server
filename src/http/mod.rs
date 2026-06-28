use crate::{admin, auth, catalog::Catalog, config::PontemeshHome, setup, web_assets};
use axum::{
    Router, middleware,
    routing::{any, get, post},
};
use std::time::Instant;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct AppState {
    pub paths: PontemeshHome,
    pub setup: setup::SetupState,
    pub auth: auth::AuthState,
    pub catalog: Catalog,
    pub started_at: Instant,
}

pub fn router(paths: PontemeshHome, setup: setup::SetupState, catalog: Catalog) -> Router {
    let state = AppState {
        paths,
        setup,
        auth: auth::AuthState::new(),
        catalog,
        started_at: Instant::now(),
    };

    let admin_routes = Router::new()
        .route(
            "/api/admin/dashboard/summary",
            get(admin::dashboard_summary),
        )
        .route("/api/admin/instance", get(admin::instance_summary))
        .route("/api/admin/system/resources", get(admin::system_resources))
        .route("/api/admin/storage/status", get(admin::storage_status))
        .route(
            "/api/admin/buckets",
            get(admin::list_buckets).post(admin::create_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}",
            get(admin::get_bucket).delete(admin::delete_bucket),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects",
            get(admin::list_objects).post(admin::upload_object),
        )
        .route(
            "/api/admin/buckets/{bucket_name}/objects/{*object_key}",
            get(admin::get_object).delete(admin::delete_object),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_admin_session,
        ));

    Router::new()
        .route("/api/setup/status", get(setup::routes::status))
        .route("/api/setup/unlock", post(setup::routes::unlock))
        .route("/api/setup/complete", post(setup::routes::complete))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .merge(admin_routes)
        .route("/api/{*path}", any(setup::routes::api_not_found))
        .fallback(web_assets::serve)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
