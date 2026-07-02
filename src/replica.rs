use crate::{
    audit,
    auth::ReplicaIdentity,
    catalog::{ReplicaHealthReportInput, ReplicaMetricInput},
    config,
    http::AppState,
};
use anyhow::{Context, bail};
use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::PathBuf, time::Duration};
use tokio::{
    fs as tokio_fs,
    io::{AsyncReadExt, AsyncSeekExt},
};

type HmacSha256 = Hmac<Sha256>;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplicaLocalState {
    objects: HashMap<String, LocalObjectState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalObjectState {
    bucket: String,
    key: String,
    manifest_id: String,
    sha256: String,
    size_bytes: i64,
    content_type: String,
    fragments: HashMap<String, LocalFragmentState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalFragmentState {
    index: i64,
    sha256: String,
    size_bytes: i64,
    path: String,
}

struct ServedLocalObject {
    manifest_id: String,
    sha256: String,
    content_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevalidateResponse {
    valid: bool,
    manifest_id: String,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedRange {
    start: u64,
    end: u64,
}

pub async fn serve_access_package_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
) -> Response {
    serve_access_package_object_inner(state, headers, package_id, bucket_name, object_key, true)
        .await
}

pub async fn head_access_package_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
) -> Response {
    serve_access_package_object_inner(state, headers, package_id, bucket_name, object_key, false)
        .await
}

async fn serve_access_package_object_inner(
    state: AppState,
    headers: HeaderMap,
    package_id: String,
    bucket_name: String,
    object_key: String,
    include_body: bool,
) -> Response {
    if let Err(error) =
        config::require_instance_role(&state.paths, config::InstanceRole::ReplicaEdge)
    {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
            .into_response();
    }

    let Some(package_token) = read_bearer_token(&headers) else {
        record_local_audit(
            &state,
            "replica_access_denied",
            Some(&package_id),
            "failure",
            &format!("bucket={bucket_name}; key={object_key}; reason=missing_token"),
        )
        .await;
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "access package bearer token required".to_owned(),
            }),
        )
            .into_response();
    };

    let replica_config = match config::load_replica_runtime_config(&state.paths) {
        Ok(config) => config,
        Err(error) => return internal_error(error),
    };

    let client = match Client::builder().timeout(Duration::from_secs(15)).build() {
        Ok(client) => client,
        Err(error) => return internal_error(anyhow::Error::new(error)),
    };

    let revalidation = match revalidate_with_origin(
        &client,
        &replica_config.origin_base_url,
        &package_id,
        &package_token,
        &bucket_name,
        &object_key,
    )
    .await
    {
        Ok(revalidation) if revalidation.valid => revalidation,
        Ok(_) => {
            record_local_audit(
                &state,
                "replica_access_denied",
                Some(&package_id),
                "failure",
                &format!("bucket={bucket_name}; key={object_key}; reason=revalidation_denied"),
            )
            .await;
            return forbidden("access package is invalid, expired or revoked");
        }
        Err(error) => {
            record_local_audit(
                &state,
                "replica_access_denied",
                Some(&package_id),
                "failure",
                &format!("bucket={bucket_name}; key={object_key}; reason={error}"),
            )
            .await;
            return forbidden("access package could not be revalidated");
        }
    };

    let storage_path = match config::configured_storage_dir(&state.paths) {
        Ok(path) => path,
        Err(error) => return internal_error(error),
    };
    let object = match load_local_object(&storage_path, &bucket_name, &object_key).await {
        Ok(Some(object)) => object,
        Ok(None) => {
            record_local_audit(
                &state,
                "replica_object_missing",
                Some(&package_id),
                "failure",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "object is not synchronized locally".to_owned(),
                }),
            )
                .into_response();
        }
        Err(error) => {
            record_local_audit(
                &state,
                "replica_object_missing",
                Some(&package_id),
                "failure",
                &format!("bucket={bucket_name}; key={object_key}; reason={error}"),
            )
            .await;
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "local replica object is not readable".to_owned(),
                }),
            )
                .into_response();
        }
    };

    if object.manifest_id != revalidation.manifest_id {
        record_local_audit(
            &state,
            "replica_access_denied",
            Some(&package_id),
            "failure",
            &format!("bucket={bucket_name}; key={object_key}; reason=manifest_mismatch"),
        )
        .await;
        return forbidden("local replica object does not match authorized manifest");
    }

    let total_size = object.bytes.len() as u64;
    let range = match headers.get(header::RANGE) {
        Some(raw) => {
            let raw = match raw.to_str() {
                Ok(raw) => raw,
                Err(_) => return range_not_satisfiable(total_size),
            };
            match parse_range(raw, total_size) {
                Ok(range) => Some(range),
                Err(_) => return range_not_satisfiable(total_size),
            }
        }
        None => None,
    };

    let (status, body) = if let Some(range) = range {
        let start = usize::try_from(range.start).unwrap_or(usize::MAX);
        let end = usize::try_from(range.end).unwrap_or(usize::MAX);
        (
            StatusCode::PARTIAL_CONTENT,
            object.bytes[start..=end].to_vec(),
        )
    } else {
        (StatusCode::OK, object.bytes)
    };
    let bytes_served = body.len();
    let response = replica_object_response(
        status,
        &object.content_type,
        &object.sha256,
        &package_id,
        total_size,
        range,
        if include_body { body } else { Vec::new() },
        bytes_served,
    );

    let event = if range.is_some() {
        "replica_range_served"
    } else {
        "replica_object_served"
    };
    record_local_audit(
        &state,
        event,
        Some(&package_id),
        "success",
        &format!(
            "bucket={bucket_name}; key={object_key}; status={}; bytes={bytes_served}; range={}",
            status.as_u16(),
            range
                .map(|range| format!("{}-{}", range.start, range.end))
                .unwrap_or_else(|| "none".to_owned())
        ),
    )
    .await;
    report_served_metrics(&client, &replica_config, bytes_served, range.is_some()).await;

    response
}

async fn revalidate_with_origin(
    client: &Client,
    origin_base_url: &str,
    package_id: &str,
    package_token: &str,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<RevalidateResponse> {
    let path = format!(
        "/pontemesh/access-packages/{}/revalidate/{}/{}",
        percent_encode_path_component(package_id),
        percent_encode_path_component(bucket_name),
        object_path(object_key)
    );
    let response = client
        .post(format!("{}{}", origin_base_url.trim_end_matches('/'), path))
        .bearer_auth(package_token)
        .send()
        .await
        .context("failed to revalidate access package with Origin")?;
    if !response.status().is_success() {
        bail!("Origin rejected access package revalidation");
    }
    response
        .json::<RevalidateResponse>()
        .await
        .context("failed to decode Origin revalidation response")
}

async fn load_local_object(
    storage_path: &std::path::Path,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<Option<ServedLocalObject>> {
    let state_path = storage_path.join("replica").join("state.json");
    if !state_path.exists() {
        return Ok(None);
    }
    let bytes = tokio_fs::read(&state_path)
        .await
        .with_context(|| format!("failed to read {}", state_path.display()))?;
    let state: ReplicaLocalState =
        serde_json::from_slice(&bytes).context("failed to parse replica local state")?;
    let Some(local) = state.objects.get(&format!("{bucket_name}/{object_key}")) else {
        return Ok(None);
    };
    if local.bucket != bucket_name || local.key != object_key || local.size_bytes < 0 {
        bail!("replica local state does not match requested object");
    }
    let mut fragments = local.fragments.values().collect::<Vec<_>>();
    fragments.sort_by_key(|fragment| fragment.index);
    if fragments.is_empty() {
        return Ok(None);
    }

    let mut object_bytes = Vec::new();
    for fragment in fragments {
        if fragment.size_bytes < 0 {
            bail!("replica local fragment size is invalid");
        }
        let path = PathBuf::from(&fragment.path);
        let fragment_bytes = tokio_fs::read(&path)
            .await
            .with_context(|| format!("failed to read local replica fragment {}", path.display()))?;
        let actual_size =
            i64::try_from(fragment_bytes.len()).context("local replica fragment is too large")?;
        if actual_size != fragment.size_bytes {
            bail!("local replica fragment size mismatch");
        }
        let actual_hash = format!("{:x}", Sha256::digest(&fragment_bytes));
        if actual_hash != fragment.sha256 {
            bail!("local replica fragment hash mismatch");
        }
        object_bytes.extend_from_slice(&fragment_bytes);
    }
    let actual_size =
        i64::try_from(object_bytes.len()).context("local replica object is too large")?;
    if actual_size != local.size_bytes {
        bail!("local replica object size mismatch");
    }
    let actual_hash = format!("{:x}", Sha256::digest(&object_bytes));
    if actual_hash != local.sha256 {
        bail!("local replica object hash mismatch");
    }

    Ok(Some(ServedLocalObject {
        manifest_id: local.manifest_id.clone(),
        sha256: local.sha256.clone(),
        content_type: local.content_type.clone(),
        bytes: object_bytes,
    }))
}

fn replica_object_response(
    status: StatusCode,
    content_type: &str,
    sha256: &str,
    package_id: &str,
    total_size: u64,
    range: Option<ResolvedRange>,
    body: Vec<u8>,
    content_length: usize,
) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, content_length.to_string())
        .header(header::ETAG, format!("\"{sha256}\""))
        .header(header::ACCEPT_RANGES, "bytes")
        .header("x-pontemesh-source", "replica-edge")
        .header("x-pontemesh-package-id", package_id);
    if let Some(range) = range {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start, range.end, total_size),
        );
    }
    builder
        .body(Body::from(body))
        .expect("valid replica object response")
}

fn range_not_satisfiable(total_size: u64) -> Response {
    Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_RANGE, format!("bytes */{total_size}"))
        .body(Body::empty())
        .expect("valid range not satisfiable response")
}

async fn record_local_audit(
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

async fn report_served_metrics(
    client: &Client,
    config: &config::ReplicaRuntimeConfig,
    bytes_served: usize,
    range: bool,
) {
    let path = format!("/pontemesh/replicas/{}/metrics", config.replica_id);
    let bytes_served = i64::try_from(bytes_served).unwrap_or(i64::MAX);
    let body = serde_json::json!({
        "bytesServed": bytes_served,
        "fragmentsServed": if range { 1 } else { 0 },
        "bytesSynced": 0,
        "fragmentsSynced": 0,
        "syncFailures": 0,
        "authFailures": 0
    });
    let timestamp = chrono::Utc::now().to_rfc3339();
    let nonce = uuid::Uuid::new_v4().to_string();
    let signature = replica_signature("POST", &path, &timestamp, &nonce, &config.replica_token);
    let _ = client
        .request(Method::POST, format!("{}{}", config.origin_base_url, path))
        .bearer_auth(&config.replica_token)
        .header("x-pontemesh-date", timestamp)
        .header("x-pontemesh-nonce", nonce)
        .header("x-pontemesh-signature", signature)
        .json(&body)
        .send()
        .await;
}

fn replica_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    token: &str,
) -> String {
    let signing_payload = format!("{method}\n{path}\n{timestamp}\n{nonce}");
    hex_hmac(token.as_bytes(), signing_payload.as_bytes())
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

fn read_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
}

fn object_path(key: &str) -> String {
    key.split('/')
        .map(percent_encode_path_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_path_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| {
            let allowed = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
            if allowed {
                vec![byte as char]
            } else {
                format!("%{byte:02X}").chars().collect()
            }
        })
        .collect()
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

fn internal_error(error: anyhow::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
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
