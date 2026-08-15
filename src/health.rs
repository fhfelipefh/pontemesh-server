use crate::{config, http::AppState};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LivenessResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub checks: ReadinessChecks,
}

#[derive(Debug, Serialize)]
pub struct ReadinessChecks {
    pub database: &'static str,
    pub storage: &'static str,
}

pub async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse { status: "UP" })
}

pub async fn readiness(State(state): State<AppState>) -> Response {
    let db_ok = state.catalog.database_connected().await;
    let storage_ok = config::configured_storage_dir(&state.paths)
        .map(|path| path.exists())
        .unwrap_or(false);

    let status = if db_ok && storage_ok { "UP" } else { "DOWN" };
    let body = ReadinessResponse {
        status,
        checks: ReadinessChecks {
            database: if db_ok { "UP" } else { "DOWN" },
            storage: if storage_ok { "UP" } else { "DOWN" },
        },
    };

    let http_status = if status == "UP" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (http_status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn liveness_returns_up() {
        let response = liveness().await;
        assert_eq!(response.status, "UP");
    }
}
