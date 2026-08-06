use crate::{
    catalog::McpTokenAuthorization,
    http::AppState,
    mcp::{auth, config, prompts, protocol, resources, tools},
};
use axum::{
    Json,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, StatusCode},
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
    info!(request_id = %request_id, "mcp_request_started");

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

    let authorization = if settings.require_auth {
        match auth::authorize_request(&state, &headers).await {
            Ok(authorization) => authorization,
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
                return protocol::http_json_rpc_error(
                    StatusCode::UNAUTHORIZED,
                    -32000,
                    error.to_string(),
                );
            }
        }
    } else {
        return protocol::http_json_rpc_error(
            StatusCode::FORBIDDEN,
            -32001,
            "MCP authentication must remain enabled",
        );
    };

    match state
        .catalog
        .count_recent_mcp_activity(&authorization.id, MCP_RATE_LIMIT_WINDOW_SECONDS)
        .await
    {
        Ok(count) if count >= MCP_RATE_LIMIT_MAX_REQUESTS => {
            let _ = state
                .catalog
                .record_mcp_activity(
                    Some(&authorization.id),
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

    let method = request.method.clone();
    let id = request.id.clone();
    let result = handle_json_rpc(&state, &settings, &authorization, &request).await;
    let duration_ms = started.elapsed().as_millis() as i64;
    match result {
        Ok(Some(value)) => {
            let _ = state
                .catalog
                .record_mcp_activity(
                    Some(&authorization.id),
                    &method,
                    activity_target(&request).as_deref(),
                    "success",
                    json!({ "requestId": request_id, "durationMs": duration_ms }),
                )
                .await;
            info!(
                request_id = %request_id,
                method = %method,
                token_id = %authorization.id,
                token_name = %authorization.name,
                duration_ms,
                status = "success",
                "mcp_request_completed"
            );
            Json(protocol::success(id, value)).into_response()
        }
        Ok(None) => StatusCode::ACCEPTED.into_response(),
        Err(error) => {
            let _ = state
                .catalog
                .record_mcp_activity(
                    Some(&authorization.id),
                    &method,
                    activity_target(&request).as_deref(),
                    "error",
                    json!({ "requestId": request_id, "durationMs": duration_ms, "error": error.to_string() }),
                )
                .await;
            Json(protocol::error(id, -32603, error.to_string())).into_response()
        }
    }
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
    authorization: &McpTokenAuthorization,
    request: &protocol::JsonRpcRequest,
) -> anyhow::Result<Option<Value>> {
    match request.method.as_str() {
        "initialize" => Ok(Some(protocol::initialize_result())),
        "notifications/initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => {
            if !settings.read_tools_enabled {
                anyhow::bail!("MCP read tools are disabled");
            }
            Ok(Some(tools::list_tools(settings, &authorization.scopes)))
        }
        "tools/call" => {
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
            Ok(Some(tools::call_tool(state, name, arguments).await?))
        }
        "resources/list" => {
            if !settings.expose_resources {
                anyhow::bail!("MCP resources are disabled");
            }
            Ok(Some(resources::list_resources(
                settings,
                &authorization.scopes,
            )))
        }
        "resources/read" => {
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
            if !settings.expose_prompts {
                anyhow::bail!("MCP prompts are disabled");
            }
            Ok(Some(prompts::list_prompts()))
        }
        "prompts/get" => {
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
