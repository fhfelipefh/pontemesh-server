use crate::{auth, http::AppState, security::token::hash_session_token};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};

static WEB_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/web/dist");

pub async fn serve(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let path = normalize_path(uri.path());

    let has_admin_session = match auth::read_auth_session(&headers) {
        Some(token) => state
            .catalog
            .find_admin_session_by_token_hash(&hash_session_token(&token))
            .await
            .ok()
            .flatten()
            .is_some_and(|session| session.role == "admin"),
        None => false,
    };

    if route_requires_admin_session(path) && !has_admin_session {
        return Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/login")
            .body(Body::empty())
            .expect("valid redirect response");
    }

    if let Some(file) = WEB_DIST.get_file(path) {
        return file_response(path, file.contents());
    }

    if should_fallback_to_index(path) {
        if let Some(index) = WEB_DIST.get_file("index.html") {
            return file_response("index.html", index.contents());
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

fn normalize_path(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

fn should_fallback_to_index(path: &str) -> bool {
    path.is_empty() || (!path.starts_with("assets/") && !path.contains('.'))
}

fn route_requires_admin_session(path: &str) -> bool {
    matches!(
        path.split('/').next(),
        Some("dashboard" | "buckets" | "objects" | "replicas" | "metrics" | "settings")
    )
}

fn file_response(path: &str, bytes: &'static [u8]) -> Response {
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_owned();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .expect("valid embedded asset response")
}
