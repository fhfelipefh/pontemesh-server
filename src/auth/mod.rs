use crate::{
    audit,
    http::AppState,
    security::{
        password::verify_admin_password,
        random::secure_url_token,
        token::{hash_bearer_token, hash_session_token},
    },
};
use axum::{
    Json,
    body::Bytes,
    extract::{OriginalUri, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{net::IpAddr, time::Duration};

const AUTH_SESSION_COOKIE: &str = "pm_admin_session";
const REPLICA_SIGNATURE_WINDOW_SECONDS: i64 = 300;
const LOGIN_RATE_LIMIT_WINDOW_SECONDS: i64 = 300;
const LOGIN_RATE_LIMIT_MAX_FAILURES: i64 = 10;
const LOGIN_MINIMUM_RESPONSE_DELAY: Duration = Duration::from_secs(2);
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct AdminSession {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct ApplicationIdentity {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReplicaIdentity {
    pub id: String,
    pub name: String,
    pub allowed_buckets: Vec<String>,
    pub revoked: bool,
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
    role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn login(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let payload: LoginRequest = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(error) => return bad_request(anyhow::anyhow!("invalid JSON payload: {error}")),
    };
    tokio::time::sleep(LOGIN_MINIMUM_RESPONSE_DELAY).await;
    let username = payload.username.trim();

    match state
        .catalog
        .count_recent_login_failures(username, LOGIN_RATE_LIMIT_WINDOW_SECONDS)
        .await
    {
        Ok(count) if count >= LOGIN_RATE_LIMIT_MAX_FAILURES => {
            record_auth_audit(
                &state,
                "login_rate_limited",
                Some(username),
                "rejected",
                "too many recent login failures",
            )
            .await;
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    error: "too many login attempts; try again later".to_owned(),
                }),
            )
                .into_response();
        }
        Ok(_) => {}
        Err(error) => return internal_error(error),
    }

    let user = match state.catalog.find_active_user_by_username(username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            audit::failure(
                "login_failed",
                Some(username),
                "unknown or inactive username",
            );
            record_auth_audit(
                &state,
                "login_failed",
                Some(username),
                "failure",
                "unknown or inactive username",
            )
            .await;
            return unauthorized("invalid username or password");
        }
        Err(error) => return internal_error(error),
    };

    if user.role != "admin" && user.role != "user" {
        audit::failure("login_failed", Some(username), "invalid role rejected");
        record_auth_audit(
            &state,
            "login_failed",
            Some(username),
            "failure",
            "invalid role rejected",
        )
        .await;
        return unauthorized("invalid username or password");
    }

    match verify_admin_password(&payload.password, &user.password_hash) {
        Ok(true) => {
            let token = secure_url_token("", 32);
            let token_hash = hash_session_token(&token);
            if let Err(error) = state
                .catalog
                .create_user_session(
                    &user.id,
                    &token_hash,
                    user_agent(&headers),
                    client_ip(&headers),
                )
                .await
            {
                return internal_error(error);
            }

            audit::event(
                "login_success",
                Some(&user.username),
                "success",
                "session created",
            );
            record_auth_audit(
                &state,
                "login_success",
                Some(&user.username),
                "success",
                "session created",
            )
            .await;
            (
                StatusCode::OK,
                [(header::SET_COOKIE, session_cookie(&headers, &token))],
                Json(AuthUserResponse {
                    authenticated: true,
                    username: Some(user.username),
                    role: Some(user.role),
                }),
            )
                .into_response()
        }
        Ok(false) => {
            audit::failure("login_failed", Some(username), "invalid password");
            record_auth_audit(
                &state,
                "login_failed",
                Some(username),
                "failure",
                "invalid password",
            )
            .await;
            unauthorized("invalid username or password")
        }
        Err(error) => internal_error(error),
    }
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = read_auth_session(&headers);
    let session = match session_from_cookie(&state, token.as_deref()).await {
        Ok(session) => session,
        Err(error) => return internal_error(error),
    };
    if let Some(token) = token.as_deref() {
        if let Err(error) = state
            .catalog
            .revoke_session_by_token_hash(&hash_session_token(token))
            .await
        {
            return internal_error(error);
        }
    }

    audit::event(
        "logout",
        session.as_ref().map(|session| session.username.as_str()),
        "success",
        "session invalidated",
    );
    record_auth_audit(
        &state,
        "logout",
        session.as_ref().map(|session| session.username.as_str()),
        "success",
        "session invalidated",
    )
    .await;
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear_auth_cookie())],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match session_from_cookie(&state, read_auth_session(&headers).as_deref()).await {
        Ok(Some(session)) => Json(AuthUserResponse {
            authenticated: true,
            username: Some(session.username),
            role: Some(session.role),
        })
        .into_response(),
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(AuthUserResponse {
                authenticated: false,
                username: None,
                role: None,
            }),
        )
            .into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn require_auth_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    match session_from_cookie(&state, read_auth_session(&headers).as_deref()).await {
        Ok(Some(session)) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "authentication required".to_owned(),
            }),
        )
            .into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn require_admin_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    match session_from_cookie(&state, read_auth_session(&headers).as_deref()).await {
        Ok(Some(session)) => {
            let is_admin = session.role == "admin";
            let is_read_only = request.method() == axum::http::Method::GET || request.method() == axum::http::Method::HEAD;
            let is_self_credentials_update = request.method() == axum::http::Method::PUT 
                && request.uri().path() == "/api/admin/users/me/credentials";

            if is_admin || is_read_only || is_self_credentials_update {
                request.extensions_mut().insert(session);
                next.run(request).await
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "admin privileges required".to_owned(),
                    }),
                )
                    .into_response()
            }
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "authentication required".to_owned(),
            }),
        )
            .into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn require_application_credential(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = read_bearer_token(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "application bearer token required".to_owned(),
            }),
        )
            .into_response();
    };

    let token_hash = hash_bearer_token(&token);
    match state
        .catalog
        .find_application_by_token_hash(&token_hash)
        .await
    {
        Ok(Some(application)) => {
            request.extensions_mut().insert(ApplicationIdentity {
                id: application.id,
                name: application.name,
                scopes: application.scopes,
            });
            next.run(request).await
        }
        Ok(None) => {
            audit::failure(
                "application_auth_failed",
                None,
                "invalid or revoked application bearer token",
            );
            record_auth_audit(
                &state,
                "application_auth_failed",
                None,
                "failure",
                "invalid or revoked application bearer token",
            )
            .await;
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "invalid application bearer token".to_owned(),
                }),
            )
                .into_response()
        }
        Err(error) => internal_error(error),
    }
}

pub async fn require_replica_credential(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = read_bearer_token(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "replica bearer token required".to_owned(),
            }),
        )
            .into_response();
    };

    let token_hash = hash_bearer_token(&token);
    match state.catalog.find_replica_by_token_hash(&token_hash).await {
        Ok(Some(replica)) => {
            if replica.revoked && !is_replica_policy_update_request(&request) {
                audit::failure(
                    "replica_auth_failed",
                    Some(&replica.name),
                    "revoked replica credential",
                );
                record_auth_audit(
                    &state,
                    "replica_auth_failed",
                    Some(&replica.name),
                    "failure",
                    "revoked replica credential",
                )
                .await;
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "replica credential is revoked".to_owned(),
                    }),
                )
                    .into_response();
            }
            if let Err(message) = validate_replica_request_signature(&headers, &request, &token) {
                audit::failure("replica_auth_failed", Some(&replica.name), &message);
                record_auth_audit(
                    &state,
                    "replica_auth_failed",
                    Some(&replica.name),
                    "failure",
                    &message,
                )
                .await;
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse { error: message }),
                )
                    .into_response();
            }
            let nonce = headers
                .get("x-pontemesh-nonce")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if let Err(error) = state
                .catalog
                .record_replica_request_nonce(&replica.id, nonce)
                .await
            {
                let message = error.to_string();
                audit::failure("replica_auth_failed", Some(&replica.name), &message);
                record_auth_audit(
                    &state,
                    "replica_auth_failed",
                    Some(&replica.name),
                    "failure",
                    &message,
                )
                .await;
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse { error: message }),
                )
                    .into_response();
            }
            request.extensions_mut().insert(ReplicaIdentity {
                id: replica.id,
                name: replica.name,
                allowed_buckets: replica.allowed_buckets,
                revoked: replica.revoked,
            });
            next.run(request).await
        }
        Ok(None) => {
            audit::failure(
                "replica_auth_failed",
                None,
                "invalid or revoked replica bearer token",
            );
            record_auth_audit(
                &state,
                "replica_auth_failed",
                None,
                "failure",
                "invalid or revoked replica bearer token",
            )
            .await;
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "invalid replica bearer token".to_owned(),
                }),
            )
                .into_response()
        }
        Err(error) => internal_error(error),
    }
}

fn validate_replica_request_signature(
    headers: &HeaderMap,
    request: &Request,
    token: &str,
) -> Result<(), String> {
    let timestamp = required_header(headers, "x-pontemesh-date")?;
    let nonce = required_header(headers, "x-pontemesh-nonce")?;
    let signature = required_header(headers, "x-pontemesh-signature")?;
    if nonce.len() < 16 || nonce.len() > 128 {
        return Err("replica nonce must be between 16 and 128 characters".to_owned());
    }
    validate_replica_timestamp(timestamp)?;
    let uri = request
        .extensions()
        .get::<OriginalUri>()
        .map(|original| &original.0)
        .unwrap_or_else(|| request.uri());
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or(uri.path());
    let signing_payload = format!(
        "{}\n{}\n{}\n{}",
        request.method(),
        path_and_query,
        timestamp,
        nonce
    );
    let expected = hex_hmac(token.as_bytes(), signing_payload.as_bytes());
    if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        return Err("replica request signature could not be verified".to_owned());
    }
    Ok(())
}

fn is_replica_policy_update_request(request: &Request) -> bool {
    request.method() == axum::http::Method::GET
        && request
            .extensions()
            .get::<OriginalUri>()
            .map(|original| original.0.path())
            .unwrap_or_else(|| request.uri().path())
            .ends_with("/policy-updates")
}

fn required_header<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing {name}"))
}

fn validate_replica_timestamp(timestamp: &str) -> Result<(), String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| "invalid x-pontemesh-date".to_owned())?
        .with_timezone(&chrono::Utc);
    let skew = (chrono::Utc::now() - parsed).num_seconds().abs();
    if skew > REPLICA_SIGNATURE_WINDOW_SECONDS {
        return Err("x-pontemesh-date is outside the allowed signature window".to_owned());
    }
    Ok(())
}

fn hex_hmac(key: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    left.ct_eq(right).into()
}

pub fn read_auth_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == AUTH_SESSION_COOKIE).then(|| value.to_owned())
    })
}

async fn session_from_cookie(
    state: &AppState,
    token: Option<&str>,
) -> anyhow::Result<Option<AdminSession>> {
    let Some(token) = token else {
        return Ok(None);
    };
    let Some(session) = state
        .catalog
        .find_admin_session_by_token_hash(&hash_session_token(token))
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(AdminSession {
        user_id: session.user_id,
        username: session.username,
        role: session.role,
    }))
}

fn read_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
}

fn session_cookie(headers: &HeaderMap, token: &str) -> HeaderValue {
    let secure = request_is_https(headers) || !request_is_localhost(headers);
    let cookie = format!(
        "{AUTH_SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax{}",
        if secure { "; Secure" } else { "" }
    );
    HeaderValue::from_str(&cookie).expect("valid auth cookie")
}

pub(crate) fn clear_auth_cookie() -> HeaderValue {
    HeaderValue::from_static("pm_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

fn request_is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

fn request_is_localhost(headers: &HeaderMap) -> bool {
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(':').next().unwrap_or(value))
    else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
}

fn client_ip(_headers: &HeaderMap) -> Option<IpAddr> {
    None
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

async fn record_auth_audit(
    state: &AppState,
    event: &str,
    principal: Option<&str>,
    outcome: &str,
    detail: &str,
) {
    if let Err(error) = state
        .catalog
        .record_audit_event(event, principal, outcome, detail)
        .await
    {
        audit::failure("audit_persist_failed", principal, &error.to_string());
    }
}
