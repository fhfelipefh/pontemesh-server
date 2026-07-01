use crate::{catalog::McpTokenAuthorization, http::AppState};
use anyhow::bail;
use axum::http::{HeaderMap, header};

pub async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
) -> anyhow::Result<McpTokenAuthorization> {
    let token =
        bearer_token(headers).ok_or_else(|| anyhow::anyhow!("MCP bearer token required"))?;
    let authorization = state
        .catalog
        .authorize_mcp_token(&token)
        .await?
        .ok_or_else(|| anyhow::anyhow!("MCP token is invalid or revoked"))?;
    state
        .catalog
        .record_mcp_token_used(&authorization.id)
        .await?;
    Ok(authorization)
}

pub fn validate_origin(headers: &HeaderMap) -> anyhow::Result<()> {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(());
    };
    if origin.starts_with("http://localhost")
        || origin.starts_with("https://localhost")
        || origin.starts_with("http://127.0.0.1")
        || origin.starts_with("https://127.0.0.1")
    {
        return Ok(());
    }
    bail!("MCP Origin is not allowed");
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
}
