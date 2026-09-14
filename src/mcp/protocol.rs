use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

pub fn success(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

pub fn http_json_rpc_error(
    status: StatusCode,
    code: i64,
    message: impl Into<String>,
) -> axum::response::Response {
    (status, Json(error(None, code, message))).into_response()
}

pub fn initialize_result(requested_version: Option<&str>) -> Value {
    let version = match requested_version {
        Some("2024-11-05") | Some("2026-07-28") => requested_version.unwrap(),
        Some(v) if v.starts_with("2024-") || v.starts_with("2025-") || v.starts_with("2026-") => v,
        _ => "2024-11-05",
    };
    json!({
        "protocolVersion": version,
        "capabilities": {
            "tools": {},
            "resources": {},
            "prompts": {}
        },
        "serverInfo": {
            "name": "pontemesh-server",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_result_protocol_version() {
        assert_eq!(initialize_result(None)["protocolVersion"], "2024-11-05");
        assert_eq!(
            initialize_result(Some("2024-11-05"))["protocolVersion"],
            "2024-11-05"
        );
        assert_eq!(
            initialize_result(Some("2026-07-28"))["protocolVersion"],
            "2026-07-28"
        );
        assert_eq!(
            initialize_result(Some("unsupported-date"))["protocolVersion"],
            "2024-11-05"
        );
    }
}
