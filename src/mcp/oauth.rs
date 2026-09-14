use crate::{
    auth::read_auth_session,
    http::AppState,
    security::{password::verify_admin_password, token::hash_session_token},
};
use axum::{
    Form, Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DynamicRegistrationRequest {
    pub client_name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub scope: Option<String>,
}

pub fn resolve_base_url(state: &AppState, headers: &HeaderMap) -> String {
    resolve_base_url_paths(&state.paths, headers)
}

pub fn resolve_base_url_paths(paths: &crate::config::PontemeshHome, headers: &HeaderMap) -> String {
    if let Ok(Some(url)) = crate::config::configured_public_web_url(paths) {
        let trimmed = url.trim_end_matches('/');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let mut scheme = "https";
    if let Some(proto) = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
    {
        if proto.starts_with("http") {
            scheme = proto;
        }
    }
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("x-original-host"))
        .or_else(|| headers.get(header::HOST))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1:8443");
    format!("{scheme}://{host}")
}

pub async fn get_protected_resource_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let base_url = resolve_base_url(&state, &headers);
    let settings = match state.catalog.get_mcp_settings().await {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    if !settings.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let metadata = json!({
        "resource": format!("{base_url}/mcp"),
        "authorization_servers": [base_url],
        "scopes_supported": ["read", "write", "admin"],
        "bearer_methods_supported": ["header"]
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(metadata),
    )
        .into_response()
}

pub async fn get_authorization_server_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let base_url = resolve_base_url(&state, &headers);
    let settings = match state.catalog.get_mcp_settings().await {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    if !settings.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let metadata = json!({
        "issuer": base_url,
        "authorization_endpoint": format!("{base_url}/oauth/authorize"),
        "token_endpoint": format!("{base_url}/oauth/token"),
        "registration_endpoint": format!("{base_url}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token", "client_credentials"],
        "code_challenge_methods_supported": ["S256", "plain"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"],
        "scopes_supported": ["read", "write", "admin"],
        "authorization_response_iss_parameter_supported": true
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(metadata),
    )
        .into_response()
}

pub async fn get_authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuthorizeQuery>,
) -> Response {
    let client_id = query.client_id.unwrap_or_default();
    let redirect_uri = query.redirect_uri.unwrap_or_default();
    let scope = query.scope.unwrap_or_else(|| "read".to_string());
    let state_param = query.state.unwrap_or_default();
    let code_challenge = query.code_challenge.unwrap_or_default();
    let code_challenge_method = query.code_challenge_method.unwrap_or_default();

    if client_id.is_empty() || redirect_uri.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing client_id or redirect_uri").into_response();
    }

    let session_user = check_session_user(&state, &headers).await;
    let is_authenticated = session_user.is_some();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ponte Mesh - Autorização MCP</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 16px; box-sizing: border-box; }}
        .card {{ background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 32px; max-width: 440px; width: 100%; box-shadow: 0 10px 25px -5px rgba(0,0,0,0.5); }}
        .header {{ display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }}
        .logo {{ width: 40px; height: 40px; border-radius: 8px; background: #3b82f6; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 20px; color: white; }}
        h1 {{ font-size: 20px; margin: 0; font-weight: 600; }}
        p {{ color: #94a3b8; font-size: 14px; line-height: 1.5; margin: 12px 0; }}
        .badge {{ display: inline-block; background: #0284c7; color: #e0f2fe; padding: 4px 8px; border-radius: 6px; font-size: 12px; font-family: monospace; }}
        .field {{ margin-top: 14px; text-align: left; }}
        label {{ display: block; font-size: 12px; color: #94a3b8; margin-bottom: 6px; text-transform: uppercase; font-weight: 500; }}
        input[type="text"], input[type="password"] {{ width: 100%; box-sizing: border-box; background: #0f172a; border: 1px solid #475569; border-radius: 6px; padding: 10px 12px; color: white; font-size: 14px; }}
        input:focus {{ border-color: #3b82f6; outline: none; }}
        .buttons {{ display: flex; gap: 12px; margin-top: 24px; }}
        button {{ flex: 1; padding: 10px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; cursor: pointer; border: none; }}
        .btn-primary {{ background: #3b82f6; color: white; }}
        .btn-primary:hover {{ background: #2563eb; }}
        .btn-secondary {{ background: #334155; color: #cbd5e1; }}
        .btn-secondary:hover {{ background: #475569; }}
    </style>
</head>
<body>
    <div class="card">
        <div class="header">
            <div class="logo">PM</div>
            <div>
                <h1>Autorizar Conexão MCP</h1>
                <span class="badge">OAuth 2.0 / Gemini</span>
            </div>
        </div>
        <p>Um aplicativo de IA (ex: <strong>Gemini Spark</strong>) está solicitando acesso à interface administrativa MCP do seu servidor Ponte Mesh.</p>
        <div style="background: #0f172a; padding: 12px; border-radius: 8px; font-size: 13px; margin: 16px 0;">
            <div><strong>Cliente:</strong> <span style="color: #38bdf8;">{client_id}</span></div>
            <div style="margin-top: 6px;"><strong>Escopos:</strong> <span style="color: #4ade80;">{scope}</span></div>
        </div>
        <form method="POST" action="/oauth/authorize">
            <input type="hidden" name="client_id" value="{client_id}">
            <input type="hidden" name="redirect_uri" value="{redirect_uri}">
            <input type="hidden" name="scope" value="{scope}">
            <input type="hidden" name="state" value="{state_param}">
            <input type="hidden" name="code_challenge" value="{code_challenge}">
            <input type="hidden" name="code_challenge_method" value="{code_challenge_method}">
            {auth_fields}
            <div class="buttons">
                <button type="submit" name="action" value="deny" class="btn-secondary">Cancelar</button>
                <button type="submit" name="action" value="authorize" class="btn-primary">Autorizar</button>
            </div>
        </form>
    </div>
</body>
</html>"#,
        client_id = client_id,
        redirect_uri = redirect_uri,
        scope = scope,
        state_param = state_param,
        code_challenge = code_challenge,
        code_challenge_method = code_challenge_method,
        auth_fields = if is_authenticated {
            "".to_string()
        } else {
            r#"<div class="field">
                <label for="username">Usuário</label>
                <input type="text" id="username" name="username" required autocomplete="username">
            </div>
            <div class="field">
                <label for="password">Senha</label>
                <input type="password" id="password" name="password" required autocomplete="current-password">
            </div>"#.to_string()
        }
    );

    Html(html).into_response()
}

pub async fn post_authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<AuthorizeForm>,
) -> Response {
    if form.action.as_deref() == Some("deny") {
        let mut target = format!("{}?error=access_denied", form.redirect_uri);
        if let Some(s) = form.state {
            target.push_str(&format!("&state={s}"));
        }
        return Redirect::to(&target).into_response();
    }

    let session_user = check_session_user(&state, &headers).await;
    let (is_authenticated, authenticated_user_id) = if let Some((user_id, _)) = session_user {
        (true, Some(user_id))
    } else if let (Some(u), Some(p)) = (form.username, form.password) {
        match state.catalog.find_active_user_by_username(u.trim()).await {
            Ok(Some(user)) => {
                if verify_admin_password(&p, &user.password_hash).unwrap_or(false) {
                    (true, Some(user.id))
                } else {
                    (false, None)
                }
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    };

    if !is_authenticated {
        let mut target = format!("{}?error=unauthorized_client", form.redirect_uri);
        if let Some(s) = form.state {
            target.push_str(&format!("&state={s}"));
        }
        return Redirect::to(&target).into_response();
    }

    let scopes: Vec<String> = form
        .scope
        .as_deref()
        .unwrap_or("read")
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();

    let code = match state
        .catalog
        .create_mcp_oauth_code(
            &form.client_id,
            &form.redirect_uri,
            &scopes,
            form.code_challenge.as_deref(),
            form.code_challenge_method.as_deref(),
            authenticated_user_id.as_deref(),
        )
        .await
    {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let mut target = format!("{}?code={}", form.redirect_uri, code);
    if let Some(s) = form.state {
        target.push_str(&format!("&state={s}"));
    }
    Redirect::to(&target).into_response()
}

pub async fn post_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut params = parse_body_params(&body);

    if let Some(auth_val) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(basic) = auth_val.strip_prefix("Basic ") {
            if let Ok(decoded) = STANDARD.decode(basic.trim()) {
                if let Ok(cred_str) = String::from_utf8(decoded) {
                    if let Some((id, secret)) = cred_str.split_once(':') {
                        params.insert("client_id".to_string(), id.to_string());
                        params.insert("client_secret".to_string(), secret.to_string());
                    }
                }
            }
        }
    }

    let grant_type = params.get("grant_type").map(String::as_str).unwrap_or("");
    match grant_type {
        "authorization_code" => {
            let code = match params.get("code") {
                Some(c) => c,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_request", "error_description": "code is required"})),
                    )
                        .into_response();
                }
            };
            let client_id = params.get("client_id").map(String::as_str).unwrap_or("");
            let redirect_uri = params.get("redirect_uri").map(String::as_str).unwrap_or("");
            let code_verifier = params.get("code_verifier").map(String::as_str);

            if let Some(secret) = params.get("client_secret") {
                if !secret.is_empty() {
                    match state
                        .catalog
                        .verify_mcp_oauth_client(client_id, secret)
                        .await
                    {
                        Ok(Some(_)) => {}
                        _ => {
                            return (
                                StatusCode::UNAUTHORIZED,
                                Json(json!({"error": "invalid_client", "error_description": "client authentication failed"})),
                            )
                                .into_response();
                        }
                    }
                }
            }

            let (scopes, user_id) = match state
                .catalog
                .consume_mcp_oauth_code(code, client_id, redirect_uri, code_verifier)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_grant", "error_description": e.to_string()})),
                    )
                        .into_response();
                }
            };

            let (access_token, refresh_token, expires_in) = match state
                .catalog
                .issue_mcp_oauth_tokens(client_id, &scopes, user_id.as_deref())
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "server_error", "error_description": e.to_string()})),
                    )
                        .into_response();
                }
            };

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "access_token": access_token,
                    "token_type": "Bearer",
                    "expires_in": expires_in,
                    "refresh_token": refresh_token,
                    "scope": scopes.join(" ")
                })),
            )
                .into_response()
        }
        "client_credentials" => {
            let client_id = match params.get("client_id") {
                Some(id) => id,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_request", "error_description": "client_id required"})),
                    )
                        .into_response();
                }
            };
            let client_secret = match params.get("client_secret") {
                Some(sec) => sec,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_request", "error_description": "client_secret required"})),
                    )
                        .into_response();
                }
            };

            let client = match state
                .catalog
                .verify_mcp_oauth_client(client_id, client_secret)
                .await
            {
                Ok(Some(c)) => c,
                _ => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": "invalid_client", "error_description": "invalid client_id or client_secret"})),
                    )
                        .into_response();
                }
            };

            let scopes = if let Some(req_scope) = params.get("scope") {
                req_scope
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect()
            } else {
                client.scopes
            };

            let (access_token, refresh_token, expires_in) = match state
                .catalog
                .issue_mcp_oauth_tokens(client_id, &scopes, None)
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "server_error", "error_description": e.to_string()})),
                    )
                        .into_response();
                }
            };

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "access_token": access_token,
                    "token_type": "Bearer",
                    "expires_in": expires_in,
                    "refresh_token": refresh_token,
                    "scope": scopes.join(" ")
                })),
            )
                .into_response()
        }
        "refresh_token" => {
            let refresh_token = match params.get("refresh_token") {
                Some(rt) => rt,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_request", "error_description": "refresh_token required"})),
                    )
                        .into_response();
                }
            };
            let client_id = params.get("client_id").map(String::as_str);

            let (access_token, new_refresh, expires_in, scopes) = match state
                .catalog
                .refresh_mcp_oauth_token(refresh_token, client_id)
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid_grant", "error_description": e.to_string()})),
                    )
                        .into_response();
                }
            };

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "access_token": access_token,
                    "token_type": "Bearer",
                    "expires_in": expires_in,
                    "refresh_token": new_refresh,
                    "scope": scopes.join(" ")
                })),
            )
                .into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "unsupported_grant_type",
                "error_description": format!("grant_type '{grant_type}' is not supported")
            })),
        )
            .into_response(),
    }
}

pub async fn post_register(
    State(state): State<AppState>,
    Json(payload): Json<DynamicRegistrationRequest>,
) -> Response {
    let name = payload
        .client_name
        .unwrap_or_else(|| "MCP Dynamic Client".to_string());
    let redirect_uris = payload.redirect_uris.unwrap_or_default();
    let scopes: Vec<String> = payload
        .scope
        .unwrap_or_else(|| "read".to_string())
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();

    match state
        .catalog
        .create_mcp_oauth_client(&name, &redirect_uris, &scopes)
        .await
    {
        Ok(created) => {
            let now = chrono::Utc::now().timestamp();
            (
                StatusCode::CREATED,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "client_id": created.client.client_id,
                    "client_secret": created.client_secret,
                    "client_id_issued_at": now,
                    "client_secret_expires_at": 0,
                    "client_name": created.client.client_name,
                    "redirect_uris": created.client.redirect_uris,
                    "scope": created.client.scopes.join(" ")
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_client_metadata",
                "error_description": e.to_string()
            })),
        )
            .into_response(),
    }
}

async fn check_session_user(state: &AppState, headers: &HeaderMap) -> Option<(String, String)> {
    let token = read_auth_session(headers)?;
    let session = state
        .catalog
        .find_admin_session_by_token_hash(&hash_session_token(&token))
        .await
        .ok()
        .flatten()?;
    Some((session.user_id, session.username))
}

fn parse_body_params(body: &[u8]) -> HashMap<String, String> {
    if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(obj) = json_val.as_object() {
            let mut map = HashMap::new();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                }
            }
            if !map.is_empty() {
                return map;
            }
        }
    }
    let query_str = String::from_utf8_lossy(body);
    let mut params = HashMap::new();
    for pair in query_str.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key), percent_decode(value));
    }
    params
}

fn percent_decode(raw: &str) -> String {
    let replaced = raw.replace('+', " ");
    let mut output = Vec::with_capacity(replaced.len());
    let bytes = replaced.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&replaced[index + 1..index + 3], 16) {
                output.push(value);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_resolve_base_url_headers() {
        let root =
            std::env::temp_dir().join(format!("pontemesh-oauth-test-{}", uuid::Uuid::new_v4()));
        let paths = crate::config::PontemeshHome::from_path(&root).expect("test home");

        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("example.com"));
        assert_eq!(
            resolve_base_url_paths(&paths, &headers),
            "https://example.com"
        );

        headers.insert("x-forwarded-proto", HeaderValue::from_static("http"));
        assert_eq!(
            resolve_base_url_paths(&paths, &headers),
            "http://example.com"
        );

        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("134.65.234.41"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert_eq!(
            resolve_base_url_paths(&paths, &headers),
            "https://134.65.234.41"
        );
    }
}
