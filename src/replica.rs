use crate::{audit, auth::ReplicaIdentity, http::AppState};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlanResponse {
    replica_id: String,
    replica_name: String,
    allowed_buckets: Vec<String>,
    generated_at: String,
    expires_at: String,
    objects: Vec<crate::catalog::ReplicaSyncObject>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn sync_plan(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path(replica_id): Path<String>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match state
        .catalog
        .list_replica_sync_objects(&replica.allowed_buckets)
        .await
    {
        Ok(objects) => {
            let generated_at = chrono::Utc::now();
            let response = SyncPlanResponse {
                replica_id: replica.id.clone(),
                replica_name: replica.name.clone(),
                allowed_buckets: replica.allowed_buckets.clone(),
                generated_at: generated_at.to_rfc3339(),
                expires_at: (generated_at + chrono::Duration::minutes(5)).to_rfc3339(),
                objects,
            };
            if let Err(error) = state
                .catalog
                .record_audit_event(
                    "replica_sync_plan_issued",
                    Some(&replica.name),
                    "success",
                    &format!("replica_id={}", replica.id),
                )
                .await
            {
                audit::failure(
                    "audit_persist_failed",
                    Some(&replica.name),
                    &error.to_string(),
                );
            }
            Json(response).into_response()
        }
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

fn forbidden(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
        .into_response()
}
