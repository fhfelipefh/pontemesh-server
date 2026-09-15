use crate::{
    catalog::McpTokenAuthorization,
    http::AppState,
    mcp::{auth, config, prompts, protocol, resources, tools},
};
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::time::Instant;
use tracing::{info, warn};

const MCP_RATE_LIMIT_WINDOW_SECONDS: i64 = 60;
const MCP_RATE_LIMIT_MAX_REQUESTS: i64 = 120;
const MCP_MAX_JSON_RPC_BYTES: usize = 2 * 1024 * 1024;

pub async fn post_mcp(State(state): State<AppState>, headers: HeaderMap, body: Body) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = Instant::now();
    info!(
        request_id = %request_id,
        host = ?headers.get(header::HOST).and_then(|v| v.to_str().ok()),
        x_forwarded_host = ?headers.get("x-forwarded-host").and_then(|v| v.to_str().ok()),
        x_forwarded_proto = ?headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()),
        user_agent = ?headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()),
        "mcp_request_started"
    );

    let settings = match state.catalog.get_mcp_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            return protocol::http_json_rpc_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                -32603,
                error.to_string(),
            );
        }
    };
    if !settings.enabled || settings.endpoint_path != config::DEFAULT_ENDPOINT_PATH {
        let _ = state
            .catalog
            .record_mcp_activity(
                None,
                "disabled",
                None,
                "rejected",
                json!({ "requestId": request_id }),
            )
            .await;
        warn!(request_id = %request_id, "mcp_disabled_request_rejected");
        return StatusCode::NOT_FOUND.into_response();
    }
    if settings.allow_localhost_only {
        if let Err(error) = auth::validate_origin(&headers) {
            let _ = state
                .catalog
                .record_mcp_activity(
                    None,
                    "origin",
                    None,
                    "rejected",
                    json!({ "requestId": request_id }),
                )
                .await;
            return protocol::http_json_rpc_error(StatusCode::FORBIDDEN, -32001, error.to_string());
        }
    }

    let bytes = match to_bytes(body, MCP_MAX_JSON_RPC_BYTES).await {
        Ok(body) => body,
        Err(error) => {
            return protocol::http_json_rpc_error(
                StatusCode::BAD_REQUEST,
                -32700,
                error.to_string(),
            );
        }
    };
    let request: protocol::JsonRpcRequest = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(error) => {
            return protocol::http_json_rpc_error(
                StatusCode::BAD_REQUEST,
                -32700,
                error.to_string(),
            );
        }
    };
    if request.jsonrpc.as_deref() != Some("2.0") {
        return Json(protocol::error(request.id, -32600, "jsonrpc must be 2.0")).into_response();
    }

    let has_auth_header = headers.contains_key(header::AUTHORIZATION);
    let is_handshake_method = matches!(
        request.method.as_str(),
        "initialize" | "notifications/initialized" | "ping"
    );

    let authorization = if settings.require_auth {
        if !has_auth_header && is_handshake_method {
            None
        } else {
            match auth::authorize_request(&state, &headers, &settings.auth_mode).await {
                Ok(authorization) => Some(authorization),
                Err(error) => {
                    let _ = state
                        .catalog
                        .record_mcp_activity(
                            None,
                            "auth",
                            None,
                            "failed",
                            json!({ "requestId": request_id }),
                        )
                        .await;
                    warn!(request_id = %request_id, "mcp_auth_failed");
                    let base_url = crate::mcp::oauth::resolve_base_url(&state, &headers);
                    let www_auth = format!(
                        "Bearer realm=\"mcp\", resource_metadata=\"{base_url}/.well-known/oauth-protected-resource/mcp\", scope=\"read\""
                    );
                    return (
                        StatusCode::UNAUTHORIZED,
                        [
                            (header::WWW_AUTHENTICATE, www_auth),
                            (header::CONTENT_TYPE, "application/json".to_string()),
                        ],
                        Json(protocol::error(request.id, -32000, error.to_string())),
                    )
                        .into_response();
                }
            }
        }
    } else {
        return protocol::http_json_rpc_error(
            StatusCode::FORBIDDEN,
            -32001,
            "MCP authentication must remain enabled",
        );
    };

    if let Some(auth) = &authorization {
        match state
            .catalog
            .count_recent_mcp_activity(&auth.id, MCP_RATE_LIMIT_WINDOW_SECONDS)
            .await
        {
            Ok(count) if count >= MCP_RATE_LIMIT_MAX_REQUESTS => {
                let _ = state
                    .catalog
                    .record_mcp_activity(
                        Some(&auth.id),
                        "rate_limit",
                        None,
                        "rejected",
                        json!({ "requestId": request_id }),
                    )
                    .await;
                return protocol::http_json_rpc_error(
                    StatusCode::TOO_MANY_REQUESTS,
                    -32002,
                    "MCP rate limit exceeded",
                );
            }
            Ok(_) => {}
            Err(error) => {
                return protocol::http_json_rpc_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    -32603,
                    error.to_string(),
                );
            }
        }
    }

    let header_protocol_version = headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok());
    let requested_version = request
        .params
        .as_ref()
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
        .or(header_protocol_version);
    let protocol_version = match requested_version {
        Some("2024-11-05") | Some("2026-07-28") => requested_version.unwrap(),
        Some(v) if v.starts_with("2024-") || v.starts_with("2025-") || v.starts_with("2026-") => v,
        _ => "2024-11-05",
    };
    let session_id = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let method = request.method.clone();
    let id = request.id.clone();
    let result = handle_json_rpc(&state, &settings, authorization.as_ref(), &request).await;
    let duration_ms = started.elapsed().as_millis() as i64;
    match result {
        Ok(Some(value)) => {
            let _ = state
                .catalog
                .record_mcp_activity(
                    authorization.as_ref().map(|a| a.id.as_str()),
                    &method,
                    activity_target(&request).as_deref(),
                    "success",
                    json!({ "requestId": request_id, "durationMs": duration_ms }),
                )
                .await;
            info!(
                request_id = %request_id,
                method = %method,
                token_id = %authorization.as_ref().map(|a| a.id.to_string()).unwrap_or_else(|| "none".to_string()),
                token_name = %authorization.as_ref().map(|a| a.name.as_str()).unwrap_or("unauthenticated"),
                duration_ms,
                status = "success",
                "mcp_request_completed"
            );
            (
                StatusCode::OK,
                mcp_response_headers(protocol_version, &session_id, true),
                Json(protocol::success(id, value)),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::ACCEPTED,
            mcp_response_headers(protocol_version, &session_id, false),
        )
            .into_response(),
        Err(error) => {
            let _ = state
                .catalog
                .record_mcp_activity(
                    authorization.as_ref().map(|a| a.id.as_str()),
                    &method,
                    activity_target(&request).as_deref(),
                    "error",
                    json!({ "requestId": request_id, "durationMs": duration_ms, "error": error.to_string() }),
                )
                .await;
            (
                StatusCode::OK,
                mcp_response_headers(protocol_version, &session_id, true),
                Json(protocol::error(id, -32603, error.to_string())),
            )
                .into_response()
        }
    }
}

pub async fn get_mcp(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let settings = match state.catalog.get_mcp_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            return protocol::http_json_rpc_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                -32603,
                error.to_string(),
            );
        }
    };
    if !settings.enabled || settings.endpoint_path != config::DEFAULT_ENDPOINT_PATH {
        return StatusCode::NOT_FOUND.into_response();
    }
    let base_url = crate::mcp::oauth::resolve_base_url(&state, &headers);
    let www_auth = format!(
        "Bearer realm=\"mcp\", resource_metadata=\"{base_url}/.well-known/oauth-protected-resource/mcp\", scope=\"read\""
    );

    let authorization = match auth::authorize_request(&state, &headers, &settings.auth_mode).await {
        Ok(auth) => auth,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                [
                    (header::WWW_AUTHENTICATE, www_auth),
                    (header::CONTENT_TYPE, "application/json".to_string()),
                ],
                Json(json!({
                    "error": "unauthorized",
                    "message": "Authentication required. See WWW-Authenticate header.",
                    "resource_metadata": format!("{base_url}/.well-known/oauth-protected-resource/mcp")
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json".to_string())],
        Json(json!({
            "status": "connected",
            "protocol": "streamable-http",
            "client": authorization.name,
            "scopes": authorization.scopes
        })),
    )
        .into_response()
}

pub async fn method_not_allowed() -> Response {
    protocol::http_json_rpc_error(
        StatusCode::METHOD_NOT_ALLOWED,
        -32601,
        "MCP Streamable HTTP currently accepts JSON-RPC messages with POST",
    )
}

async fn handle_json_rpc(
    state: &AppState,
    settings: &crate::catalog::McpSettings,
    authorization: Option<&McpTokenAuthorization>,
    request: &protocol::JsonRpcRequest,
) -> anyhow::Result<Option<Value>> {
    match request.method.as_str() {
        "initialize" => {
            let requested_version = request
                .params
                .as_ref()
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str);
            Ok(Some(protocol::initialize_result(requested_version)))
        }
        "notifications/initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => {
            let authorization = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.read_tools_enabled {
                anyhow::bail!("MCP read tools are disabled");
            }
            Ok(Some(tools::list_tools(settings, &authorization.scopes)))
        }
        "tools/call" => {
            let authorization = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.read_tools_enabled {
                anyhow::bail!("MCP read tools are disabled");
            }
            let params = request.params.clone().unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("tool name is required"))?;
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if let Some(permission) = tools::tool_permission(name) {
                if !tools::is_allowed(permission, settings, &authorization.scopes) {
                    anyhow::bail!("MCP token is not allowed to call tool {name}");
                }
            }
            Ok(Some(
                tools::call_tool(state, authorization, name, arguments).await?,
            ))
        }
        "resources/list" => {
            let authorization = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.expose_resources {
                anyhow::bail!("MCP resources are disabled");
            }
            Ok(Some(resources::list_resources(
                settings,
                &authorization.scopes,
            )))
        }
        "resources/read" => {
            let authorization = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.expose_resources {
                anyhow::bail!("MCP resources are disabled");
            }
            let params = request.params.clone().unwrap_or_else(|| json!({}));
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("resource uri is required"))?;
            Ok(Some(
                resources::read_resource(state, settings, &authorization.scopes, uri).await?,
            ))
        }
        "prompts/list" => {
            let _ = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.expose_prompts {
                anyhow::bail!("MCP prompts are disabled");
            }
            Ok(Some(prompts::list_prompts()))
        }
        "prompts/get" => {
            let _ = authorization.ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
            if !settings.expose_prompts {
                anyhow::bail!("MCP prompts are disabled");
            }
            let params = request.params.clone().unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("prompt name is required"))?;
            Ok(Some(prompts::get_prompt(name)?))
        }
        _ => anyhow::bail!("unsupported MCP method: {}", request.method),
    }
}

fn activity_target(request: &protocol::JsonRpcRequest) -> Option<String> {
    let params = request.params.as_ref()?;
    params
        .get("name")
        .or_else(|| params.get("uri"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn mcp_response_headers(
    protocol_version: &str,
    session_id: &str,
    include_content_type: bool,
) -> HeaderMap {
    let mut map = HeaderMap::new();
    if include_content_type {
        map.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    if let Ok(val) = HeaderValue::from_str(protocol_version) {
        map.insert(HeaderName::from_static("mcp-protocol-version"), val);
    }
    if let Ok(val) = HeaderValue::from_str(session_id) {
        map.insert(HeaderName::from_static("mcp-session-id"), val);
    }
    map
}
