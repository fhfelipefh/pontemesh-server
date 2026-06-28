use crate::{
    audit, config,
    http::AppState,
    security::{password::verify_admin_password, random::secure_url_token},
};
use axum::{
    Json,
    body::Bytes,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

const AUTH_SESSION_COOKIE: &str = "pm_admin_session";

#[derive(Debug, Clone)]
pub struct AuthState {
    sessions: Arc<Mutex<HashMap<String, AdminSession>>>,
}

#[derive(Debug, Clone)]
pub struct AdminSession {
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserResponse {
    authenticated: bool,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_session(&self, username: String) -> String {
        let token = secure_url_token("", 32);
        self.sessions
            .lock()
            .expect("poisoned auth session lock")
            .insert(token.clone(), AdminSession { username });
        token
    }

    pub fn get_session(&self, token: Option<&str>) -> Option<AdminSession> {
        let token = token?;
        self.sessions
            .lock()
            .expect("poisoned auth session lock")
            .get(token)
            .cloned()
    }

    pub fn remove_session(&self, token: Option<&str>) -> Option<AdminSession> {
        let token = token?;
        self.sessions
            .lock()
            .expect("poisoned auth session lock")
            .remove(token)
    }
}

pub async fn login(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let payload: LoginRequest = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(error) => return bad_request(anyhow::anyhow!("invalid JSON payload: {error}")),
    };
    let username = payload.username.trim();

    let config = match config::load_instance_config(&state.paths) {
        Ok(config) => config,
        Err(error) => return internal_error(error),
    };

    if username != config.admin.username {
        audit::failure("login_failed", Some(username), "unknown admin username");
        return unauthorized("invalid username or password");
    }

    match verify_admin_password(&payload.password, &config.admin.password_hash) {
        Ok(true) => {
            let token = state.auth.create_session(username.to_owned());
            audit::event(
                "login_success",
                Some(username),
                "success",
                "admin session created",
            );
            (
                StatusCode::OK,
                [(header::SET_COOKIE, session_cookie(&headers, &token))],
                Json(AuthUserResponse {
                    authenticated: true,
                    username: Some(username.to_owned()),
                }),
            )
                .into_response()
        }
        Ok(false) => {
            audit::failure("login_failed", Some(username), "invalid password");
            unauthorized("invalid username or password")
        }
        Err(error) => internal_error(error),
    }
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = state
        .auth
        .remove_session(read_auth_session(&headers).as_deref());
    audit::event(
        "logout",
        session.as_ref().map(|session| session.username.as_str()),
        "success",
        "admin session invalidated",
    );
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear_auth_cookie())],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match state
        .auth
        .get_session(read_auth_session(&headers).as_deref())
    {
        Some(session) => Json(AuthUserResponse {
            authenticated: true,
            username: Some(session.username),
        })
        .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(AuthUserResponse {
                authenticated: false,
                username: None,
            }),
        )
            .into_response(),
    }
}

pub async fn require_admin_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    match state
        .auth
        .get_session(read_auth_session(&headers).as_deref())
    {
        Some(session) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "authentication required".to_owned(),
            }),
        )
            .into_response(),
    }
}

pub fn read_auth_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == AUTH_SESSION_COOKIE).then(|| value.to_owned())
    })
}

fn session_cookie(headers: &HeaderMap, token: &str) -> HeaderValue {
    let secure = request_is_https(headers);
    let cookie = format!(
        "{AUTH_SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax{}",
        if secure { "; Secure" } else { "" }
    );
    HeaderValue::from_str(&cookie).expect("valid auth cookie")
}

fn clear_auth_cookie() -> HeaderValue {
    HeaderValue::from_static("pm_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

fn request_is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

fn bad_request(error: anyhow::Error) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
        .into_response()
}

fn internal_error(error: anyhow::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}
