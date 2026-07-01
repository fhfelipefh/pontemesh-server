use crate::{
    audit,
    auth::ReplicaIdentity,
    catalog::{ReplicaHealthReportInput, ReplicaMetricInput},
    http::AppState,
};
use anyhow::bail;
use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceAvailabilityRequest {
    bucket: String,
    key: String,
    endpoint: String,
    #[serde(default)]
    available_fragments: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportHealthRequest {
    status: String,
    version: Option<String>,
    storage_available_bytes: Option<i64>,
    #[serde(default)]
    error_count: i64,
    #[serde(default)]
    detail: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMetricsRequest {
    #[serde(default)]
    bytes_synced: i64,
    #[serde(default)]
    bytes_served: i64,
    #[serde(default)]
    fragments_synced: i64,
    #[serde(default)]
    fragments_served: i64,
    #[serde(default)]
    sync_failures: i64,
    #[serde(default)]
    auth_failures: i64,
    avg_latency_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyUpdatesQuery {
    since: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn sync_object(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    headers: HeaderMap,
    Path((replica_id, bucket_name, object_key)): Path<(String, String, String)>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match sync_object_inner(
        &state,
        &replica,
        &headers,
        &bucket_name,
        object_key.trim_start_matches('/'),
    )
    .await
    {
        Ok(response) => response,
        Err(error) => bad_request(error),
    }
}

pub async fn sync_fragment(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path((replica_id, manifest_id, fragment_id)): Path<(String, String, String)>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match sync_fragment_inner(&state, &replica, &manifest_id, &fragment_id).await {
        Ok(response) => response,
        Err(error) => bad_request(error),
    }
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

pub async fn announce_availability(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path(replica_id): Path<String>,
    Json(payload): Json<AnnounceAvailabilityRequest>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match state
        .catalog
        .record_replica_object_availability(
            &replica.id,
            &replica.allowed_buckets,
            &payload.bucket,
            &payload.key,
            &payload.endpoint,
            &payload.available_fragments,
        )
        .await
    {
        Ok(record) => {
            if let Err(error) = state
                .catalog
                .record_audit_event(
                    "replica_availability_announced",
                    Some(&replica.name),
                    "success",
                    &format!(
                        "replica_id={}; bucket={}; key={}",
                        replica.id, record.bucket, record.key
                    ),
                )
                .await
            {
                audit::failure(
                    "audit_persist_failed",
                    Some(&replica.name),
                    &error.to_string(),
                );
            }
            Json(record).into_response()
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

pub async fn report_health(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path(replica_id): Path<String>,
    Json(payload): Json<ReportHealthRequest>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match state
        .catalog
        .record_replica_health(
            &replica.id,
            ReplicaHealthReportInput {
                status: payload.status,
                version: payload.version,
                storage_available_bytes: payload.storage_available_bytes,
                error_count: payload.error_count,
                detail: payload.detail,
            },
        )
        .await
    {
        Ok(report) => Json(report).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn report_metrics(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path(replica_id): Path<String>,
    Json(payload): Json<ReportMetricsRequest>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match state
        .catalog
        .record_replica_metrics(
            &replica.id,
            ReplicaMetricInput {
                bytes_synced: payload.bytes_synced,
                bytes_served: payload.bytes_served,
                fragments_synced: payload.fragments_synced,
                fragments_served: payload.fragments_served,
                sync_failures: payload.sync_failures,
                auth_failures: payload.auth_failures,
                avg_latency_ms: payload.avg_latency_ms,
            },
        )
        .await
    {
        Ok(record) => Json(record).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn policy_updates(
    State(state): State<AppState>,
    Extension(replica): Extension<ReplicaIdentity>,
    Path(replica_id): Path<String>,
    Query(query): Query<PolicyUpdatesQuery>,
) -> Response {
    if replica_id != replica.id {
        return forbidden("replica credential does not match requested replica id");
    }

    match state
        .catalog
        .list_replica_policy_updates(&replica.id, query.since)
        .await
    {
        Ok(updates) => Json(updates).into_response(),
        Err(error) => bad_request(error),
    }
}

async fn sync_object_inner(
    state: &AppState,
    replica: &ReplicaIdentity,
    headers: &HeaderMap,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<Response> {
    let object = state
        .catalog
        .authorize_replica_object_sync(
            &replica.id,
            &replica.allowed_buckets,
            bucket_name,
            object_key,
        )
        .await?;
    let bytes = fs::read(&object.storage_path)
        .map_err(|_| anyhow::anyhow!("object data is unavailable"))?;
    let total_size = bytes.len() as u64;
    let (status, body, range) = if let Some(range_header) = headers.get(header::RANGE) {
        let range_header = range_header
            .to_str()
            .map_err(|_| anyhow::anyhow!("Range header is not valid UTF-8"))?;
        let range = parse_range(range_header, total_size)?;
        let start = usize::try_from(range.start)
            .map_err(|_| anyhow::anyhow!("range start is too large"))?;
        let end =
            usize::try_from(range.end).map_err(|_| anyhow::anyhow!("range end is too large"))?;
        (
            StatusCode::PARTIAL_CONTENT,
            bytes[start..=end].to_vec(),
            Some(range),
        )
    } else {
        (StatusCode::OK, bytes, None)
    };

    state
        .catalog
        .record_replica_sync_transfer(
            &replica.id,
            i64::try_from(body.len()).map_err(|_| anyhow::anyhow!("response body is too large"))?,
            if range.is_some() { 1 } else { 0 },
        )
        .await?;
    if let Err(error) = state
        .catalog
        .record_audit_event(
            "replica_object_synced",
            Some(&replica.name),
            "success",
            &format!(
                "replica_id={}; bucket={bucket_name}; key={object_key}",
                replica.id
            ),
        )
        .await
    {
        audit::failure(
            "audit_persist_failed",
            Some(&replica.name),
            &error.to_string(),
        );
    }

    Ok(object_body_response(&object, status, body, range))
}

async fn sync_fragment_inner(
    state: &AppState,
    replica: &ReplicaIdentity,
    manifest_id: &str,
    fragment_id: &str,
) -> anyhow::Result<Response> {
    let fragment = state
        .catalog
        .authorize_replica_fragment_sync(
            &replica.id,
            &replica.allowed_buckets,
            manifest_id,
            fragment_id,
        )
        .await?;
    if fragment.byte_range_start < 0
        || fragment.byte_range_end < 0
        || fragment.byte_range_start > fragment.byte_range_end
        || fragment.byte_range_end >= fragment.object.size_bytes
    {
        bail!("fragment range is invalid for object data");
    }
    let fragment_size = fragment.byte_range_end - fragment.byte_range_start + 1;
    let mut file = tokio::fs::File::open(&fragment.object.storage_path)
        .await
        .map_err(|_| anyhow::anyhow!("object data is unavailable"))?;
    file.seek(std::io::SeekFrom::Start(
        u64::try_from(fragment.byte_range_start)
            .map_err(|_| anyhow::anyhow!("fragment range start is invalid"))?,
    ))
    .await?;
    let mut body = vec![
        0;
        usize::try_from(fragment_size)
            .map_err(|_| anyhow::anyhow!("fragment size is too large"))?
    ];
    file.read_exact(&mut body).await?;
    let range = ResolvedRange {
        start: u64::try_from(fragment.byte_range_start)
            .map_err(|_| anyhow::anyhow!("fragment range start is invalid"))?,
        end: u64::try_from(fragment.byte_range_end)
            .map_err(|_| anyhow::anyhow!("fragment range end is invalid"))?,
    };
    state
        .catalog
        .record_replica_sync_transfer(
            &replica.id,
            i64::try_from(body.len()).map_err(|_| anyhow::anyhow!("response body is too large"))?,
            1,
        )
        .await?;
    state
        .catalog
        .record_fragment_transfer_event(
            "REPLICA_EDGE",
            Some(&replica.id),
            &fragment.bucket_name,
            &fragment.object_key,
            &fragment.manifest_id,
            fragment.fragment_index,
            &fragment.fragment_hash,
            "FRAGMENT_SYNCED",
            i64::try_from(body.len()).map_err(|_| anyhow::anyhow!("response body is too large"))?,
            "success",
            serde_json::json!({ "replicaId": replica.id }),
        )
        .await?;
    if let Err(error) = state
        .catalog
        .record_audit_event(
            "replica_fragment_synced",
            Some(&replica.name),
            "success",
            &format!(
                "replica_id={}; manifest_id={manifest_id}; fragment_id={fragment_id}",
                replica.id
            ),
        )
        .await
    {
        audit::failure(
            "audit_persist_failed",
            Some(&replica.name),
            &error.to_string(),
        );
    }

    Ok(object_body_response(
        &fragment.object,
        StatusCode::PARTIAL_CONTENT,
        body,
        Some(range),
    ))
}

#[derive(Debug, Clone, Copy)]
struct ResolvedRange {
    start: u64,
    end: u64,
}

fn parse_range(raw: &str, total_size: u64) -> anyhow::Result<ResolvedRange> {
    if total_size == 0 {
        bail!("cannot apply Range to empty object");
    }
    let range = raw
        .strip_prefix("bytes=")
        .ok_or_else(|| anyhow::anyhow!("only bytes ranges are supported"))?;
    if range.contains(',') {
        bail!("multiple ranges are not supported");
    }
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| anyhow::anyhow!("invalid Range header"))?;
    let (start, end) = if start.is_empty() {
        let suffix_len: u64 = end
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid suffix byte range"))?;
        if suffix_len == 0 {
            bail!("suffix byte range must be greater than zero");
        }
        (total_size.saturating_sub(suffix_len), total_size - 1)
    } else {
        let start: u64 = start
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid range start"))?;
        let end = if end.is_empty() {
            total_size - 1
        } else {
            end.parse()
                .map_err(|_| anyhow::anyhow!("invalid range end"))?
        };
        (start, end)
    };
    if start >= total_size || end >= total_size || start > end {
        bail!("requested range is not satisfiable");
    }
    Ok(ResolvedRange { start, end })
}

fn object_body_response(
    object: &crate::catalog::ObjectRecord,
    status: StatusCode,
    bytes: Vec<u8>,
    range: Option<ResolvedRange>,
) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, object.content_type.as_str())
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header("ETag", format!("\"{}\"", object.sha256))
        .header("x-pontemesh-object-state", object.state.as_str());
    if let Some(range) = range {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start, range.end, object.size_bytes),
        );
    }
    builder
        .body(Body::from(bytes))
        .expect("valid replica object response")
}

fn bad_request(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message == "requested range is not satisfiable" {
        StatusCode::RANGE_NOT_SATISFIABLE
    } else if message == "replica object synchronization is not authorized"
        || message == "replica fragment synchronization is not authorized"
        || message == "fragment does not belong to requested manifest"
    {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::BAD_REQUEST
    };
    let mut response = (status, Json(ErrorResponse { error: message })).into_response();
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }
    response
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
