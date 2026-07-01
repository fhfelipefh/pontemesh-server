use crate::{
    audit,
    auth::ApplicationIdentity,
    catalog::{
        BucketPolicy, ObjectManifest, ObjectRecord, PeerAvailabilityInput, PeerAvailabilityRecord,
        ReplicaAvailabilityRecord, SdkFragmentEventInput, SdkFragmentEventRecord,
    },
    http::AppState,
    security::token::hash_bearer_token,
};
use anyhow::bail;
use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccessPackageRequest {
    bucket: String,
    key: String,
    ttl_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPackageResponse {
    id: String,
    package_token: String,
    bucket: String,
    key: String,
    version: String,
    manifest_id: String,
    expires_at: String,
    scope: Vec<String>,
    authorized_sources: Vec<AuthorizedSource>,
    source_selection: SourceSelectionContract,
    fallback: FallbackContract,
    manifest: ManifestResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesResponse {
    bucket: String,
    key: String,
    manifest_id: String,
    authorized_sources: Vec<AuthorizedSource>,
    source_selection: SourceSelectionContract,
    fallback: FallbackContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityResponse {
    bucket: String,
    key: String,
    manifest_id: String,
    object_state: String,
    origin_available: bool,
    replica_sources: usize,
    peer_sources: usize,
    fragments: Vec<FragmentAvailability>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPolicyResponse {
    bucket: String,
    key: String,
    manifest_id: String,
    object_state: String,
    access_package_ttl_seconds: i64,
    fragment_size_bytes: i64,
    allow_replica_edge: bool,
    allow_peer_sharing: bool,
    source_selection_strategy: String,
    fragment_priority_strategy: String,
    failure_threshold: i64,
    fallback_mode: String,
    fallback_supports_range: bool,
    preserve_validated_fragments: bool,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FragmentAvailability {
    index: usize,
    fragment_id: String,
    byte_range_start: u64,
    byte_range_end: u64,
    size_bytes: usize,
    sha256: String,
    origin_available: bool,
    replica_source_ids: Vec<String>,
    peer_source_ids: Vec<String>,
    available_source_types: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevalidateAccessPackageResponse {
    package_id: String,
    bucket: String,
    key: String,
    manifest_id: String,
    valid: bool,
    authorized_sources: Vec<AuthorizedSource>,
    source_selection: SourceSelectionContract,
    fallback: FallbackContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestResponse {
    manifest_id: String,
    object_id: String,
    bucket: String,
    key: String,
    version: String,
    total_size_bytes: i64,
    content_type: String,
    object_hash_algorithm: String,
    object_sha256: String,
    fragment_size_bytes: usize,
    fragments: Vec<FragmentDescriptor>,
    availability_state: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FragmentDescriptor {
    index: usize,
    fragment_id: String,
    byte_range_start: u64,
    byte_range_end: u64,
    size_bytes: usize,
    hash_algorithm: String,
    sha256: String,
    priority: String,
    fallback_range_header: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedSource {
    id: String,
    source_type: String,
    endpoint: String,
    priority: u8,
    expires_at: String,
    available_fragments: Vec<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSelectionContract {
    strategy: String,
    fragment_priority: String,
    failure_threshold: i64,
    allow_peer_sharing: bool,
    allow_replica_edge: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackContract {
    source_type: String,
    object_endpoint: String,
    supports_range: bool,
    preserve_validated_fragments: bool,
    mode: String,
    revalidate_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncePeerRequest {
    peer_id: String,
    endpoint: String,
    available_fragments: Vec<i64>,
    ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkFragmentEventRequest {
    source_type: String,
    peer_availability_id: Option<String>,
    fragment_index: i64,
    fragment_hash: String,
    event_type: String,
    bytes_transferred: i64,
    outcome: String,
    latency_ms: Option<i64>,
    detail: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn get_manifest(
    State(state): State<AppState>,
    Extension(application): Extension<ApplicationIdentity>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    if !has_scope(&application, "pontemesh:manifest:read") {
        return forbidden("missing scope: pontemesh:manifest:read");
    }

    match load_manifest(&state, &bucket_name, object_key.trim_start_matches('/')).await {
        Ok(manifest) => {
            record_mesh_audit(
                &state,
                "manifest_issued",
                &application.name,
                "success",
                &format!("bucket={bucket_name}; key={}", manifest.key),
            )
            .await;
            Json(manifest).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn create_access_package(
    State(state): State<AppState>,
    Extension(application): Extension<ApplicationIdentity>,
    headers: HeaderMap,
    Json(payload): Json<CreateAccessPackageRequest>,
) -> Response {
    if !has_scope(&application, "pontemesh:access-package:create") {
        return forbidden("missing scope: pontemesh:access-package:create");
    }

    match create_access_package_inner(&state, &application, &headers, payload).await {
        Ok(package) => {
            record_mesh_audit(
                &state,
                "access_package_created",
                &application.name,
                "success",
                &format!(
                    "bucket={}; key={}; package_id={}",
                    package.bucket, package.key, package.id
                ),
            )
            .await;
            (StatusCode::CREATED, Json(package)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_sources(
    State(state): State<AppState>,
    Extension(application): Extension<ApplicationIdentity>,
    headers: HeaderMap,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    if !has_scope(&application, "pontemesh:sources:read") {
        return forbidden("missing scope: pontemesh:sources:read");
    }

    match get_sources_inner(
        &state,
        &headers,
        &bucket_name,
        object_key.trim_start_matches('/'),
    )
    .await
    {
        Ok(sources) => {
            record_mesh_audit(
                &state,
                "sources_issued",
                &application.name,
                "success",
                &format!("bucket={bucket_name}; key={}", sources.key),
            )
            .await;
            Json(sources).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_availability(
    State(state): State<AppState>,
    Extension(application): Extension<ApplicationIdentity>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    if !has_scope(&application, "pontemesh:availability:read") {
        return forbidden("missing scope: pontemesh:availability:read");
    }

    match get_availability_inner(&state, &bucket_name, object_key.trim_start_matches('/')).await {
        Ok(availability) => {
            record_mesh_audit(
                &state,
                "availability_issued",
                &application.name,
                "success",
                &format!("bucket={bucket_name}; key={}", availability.key),
            )
            .await;
            Json(availability).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_object_policy(
    State(state): State<AppState>,
    Extension(application): Extension<ApplicationIdentity>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    if !has_scope(&application, "pontemesh:policies:read") {
        return forbidden("missing scope: pontemesh:policies:read");
    }

    match get_object_policy_inner(&state, &bucket_name, object_key.trim_start_matches('/')).await {
        Ok(policy) => {
            record_mesh_audit(
                &state,
                "policy_issued",
                &application.name,
                "success",
                &format!("bucket={bucket_name}; key={}", policy.key),
            )
            .await;
            Json(policy).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revalidate_access_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
) -> Response {
    let Some(package_token) = read_bearer_token(&headers) else {
        return unauthorized("access package bearer token required");
    };

    match revalidate_access_package_inner(
        &state,
        &headers,
        &package_id,
        &bucket_name,
        object_key.trim_start_matches('/'),
        &package_token,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn announce_peer_availability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
    Json(payload): Json<AnnouncePeerRequest>,
) -> Response {
    let Some(package_token) = read_bearer_token(&headers) else {
        return unauthorized("access package bearer token required");
    };

    match announce_peer_availability_inner(
        &state,
        &package_id,
        &bucket_name,
        object_key.trim_start_matches('/'),
        &package_token,
        payload,
    )
    .await
    {
        Ok(record) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn record_sdk_fragment_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
    Json(payload): Json<SdkFragmentEventRequest>,
) -> Response {
    let Some(package_token) = read_bearer_token(&headers) else {
        return unauthorized("access package bearer token required");
    };

    match record_sdk_fragment_event_inner(
        &state,
        &package_id,
        &bucket_name,
        object_key.trim_start_matches('/'),
        &package_token,
        payload,
    )
    .await
    {
        Ok(record) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn get_object_with_access_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((package_id, bucket_name, object_key)): Path<(String, String, String)>,
) -> Response {
    let Some(package_token) = read_bearer_token(&headers) else {
        return unauthorized("access package bearer token required");
    };
    match get_object_with_access_package_inner(
        &state,
        &headers,
        &package_id,
        &bucket_name,
        object_key.trim_start_matches('/'),
        &package_token,
    )
    .await
    {
        Ok(response) => response,
        Err(error) => bad_request(error),
    }
}

async fn get_object_with_access_package_inner(
    state: &AppState,
    headers: &HeaderMap,
    package_id: &str,
    bucket_name: &str,
    object_key: &str,
    package_token: &str,
) -> anyhow::Result<Response> {
    let authorization = state
        .catalog
        .authorize_access_package(
            package_id,
            &hash_bearer_token(package_token),
            bucket_name,
            object_key,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("access package is invalid, expired or revoked"))?;
    let object = state
        .catalog
        .get_object_record(bucket_name, object_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("object not found"))?;
    if object.state != "AVAILABLE" {
        bail!("object is not available");
    }
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
        .record_origin_transfer(
            Some(&authorization.application_id),
            &authorization.bucket_name,
            &authorization.object_key,
            i64::try_from(body.len()).map_err(|_| anyhow::anyhow!("response body is too large"))?,
            range.map(|value| (value.start, value.end)),
            status.as_u16(),
        )
        .await?;
    record_mesh_audit(
        state,
        "access_package_object_served",
        &authorization.application_id,
        "success",
        &format!(
            "package_id={}; manifest_id={}; bucket={}; key={}",
            authorization.package_id,
            authorization.manifest_id,
            authorization.bucket_name,
            authorization.object_key
        ),
    )
    .await;

    Ok(object_body_response(&object, status, body, range))
}

async fn create_access_package_inner(
    state: &AppState,
    application: &ApplicationIdentity,
    headers: &HeaderMap,
    payload: CreateAccessPackageRequest,
) -> anyhow::Result<AccessPackageResponse> {
    let policy = state.catalog.get_bucket_policy(&payload.bucket).await?;
    let ttl_seconds = payload
        .ttl_seconds
        .unwrap_or(policy.access_package_ttl_seconds);
    if !(60..=policy.access_package_ttl_seconds).contains(&ttl_seconds) {
        bail!(
            "ttlSeconds must be between 60 and the bucket policy maximum of {}",
            policy.access_package_ttl_seconds
        );
    }

    let manifest = load_manifest_with_policy(state, &payload.bucket, &payload.key, &policy).await?;
    if manifest.availability_state != "AVAILABLE" {
        bail!("object is not available");
    }

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds);
    let record = state
        .catalog
        .create_access_package(&application.id, &payload.bucket, &payload.key, expires_at)
        .await?;
    let base_url = request_s3_base_url(headers);
    let object_endpoint = format!(
        "{base_url}/{}/{}",
        url_component(&payload.bucket),
        object_path(&payload.key)
    );
    let authorized_sources = authorized_sources_for_object(
        state,
        &payload.bucket,
        &payload.key,
        &record.expires_at,
        &object_endpoint,
        &policy,
    )
    .await?;
    let revalidate_endpoint = Some(format!(
        "{}/pontemesh/access-packages/{}/revalidate/{}/{}",
        request_base_url(headers).trim_end_matches('/'),
        url_component(&record.id),
        url_component(&payload.bucket),
        object_path(&payload.key)
    ));

    Ok(AccessPackageResponse {
        id: record.id,
        package_token: record.package_token,
        bucket: payload.bucket,
        key: payload.key,
        version: manifest.version.clone(),
        manifest_id: record.manifest_id,
        expires_at: record.expires_at.clone(),
        scope: vec![
            "object:read".to_owned(),
            "source:origin".to_owned(),
            "manifest:read".to_owned(),
        ],
        authorized_sources,
        source_selection: source_selection_contract(&policy),
        fallback: fallback_contract(&object_endpoint, &policy, revalidate_endpoint),
        manifest,
    })
}

async fn get_sources_inner(
    state: &AppState,
    headers: &HeaderMap,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<SourcesResponse> {
    let manifest = load_manifest(state, bucket_name, object_key).await?;
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let base_url = request_s3_base_url(headers);
    let object_endpoint = format!(
        "{base_url}/{}/{}",
        url_component(bucket_name),
        object_path(object_key)
    );
    let expires_at = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
    let authorized_sources = authorized_sources_for_object(
        state,
        bucket_name,
        object_key,
        &expires_at,
        &object_endpoint,
        &policy,
    )
    .await?;

    Ok(SourcesResponse {
        bucket: bucket_name.to_owned(),
        key: object_key.to_owned(),
        manifest_id: manifest.manifest_id,
        authorized_sources,
        source_selection: source_selection_contract(&policy),
        fallback: fallback_contract(&object_endpoint, &policy, None),
    })
}

async fn get_availability_inner(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<AvailabilityResponse> {
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let manifest = load_manifest_with_policy(state, bucket_name, object_key, &policy).await?;
    let replica_sources = if policy.source_selection_strategy == "ORIGIN_ONLY" {
        Vec::new()
    } else {
        state
            .catalog
            .list_authorized_replica_sources(bucket_name, object_key)
            .await?
    };
    let peer_sources =
        if policy.allow_peer_sharing && policy.source_selection_strategy != "ORIGIN_ONLY" {
            state
                .catalog
                .list_authorized_peer_sources(bucket_name, object_key)
                .await?
        } else {
            Vec::new()
        };

    let fragments = manifest
        .fragments
        .iter()
        .map(|fragment| {
            let fragment_index = i64::try_from(fragment.index)
                .map_err(|_| anyhow::anyhow!("fragment index is too large"))?;
            let replica_source_ids = replica_sources
                .iter()
                .filter(|source| source.available_fragments.contains(&fragment_index))
                .map(|source| source.replica_id.clone())
                .collect::<Vec<_>>();
            let peer_source_ids = peer_sources
                .iter()
                .filter(|source| source.available_fragments.contains(&fragment_index))
                .map(|source| source.id.clone())
                .collect::<Vec<_>>();
            let mut available_source_types = vec!["ORIGIN".to_owned()];
            if !replica_source_ids.is_empty() {
                available_source_types.push("REPLICA_EDGE".to_owned());
            }
            if !peer_source_ids.is_empty() {
                available_source_types.push("PEER".to_owned());
            }

            Ok(FragmentAvailability {
                index: fragment.index,
                fragment_id: fragment.fragment_id.clone(),
                byte_range_start: fragment.byte_range_start,
                byte_range_end: fragment.byte_range_end,
                size_bytes: fragment.size_bytes,
                sha256: fragment.sha256.clone(),
                origin_available: true,
                replica_source_ids,
                peer_source_ids,
                available_source_types,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(AvailabilityResponse {
        bucket: manifest.bucket,
        key: manifest.key,
        manifest_id: manifest.manifest_id,
        object_state: manifest.availability_state,
        origin_available: true,
        replica_sources: replica_sources.len(),
        peer_sources: peer_sources.len(),
        fragments,
    })
}

async fn get_object_policy_inner(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<ObjectPolicyResponse> {
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let manifest = load_manifest_with_policy(state, bucket_name, object_key, &policy).await?;
    Ok(ObjectPolicyResponse {
        bucket: manifest.bucket,
        key: manifest.key,
        manifest_id: manifest.manifest_id,
        object_state: manifest.availability_state,
        access_package_ttl_seconds: policy.access_package_ttl_seconds,
        fragment_size_bytes: policy.fragment_size_bytes,
        allow_replica_edge: policy.allow_replica_edge,
        allow_peer_sharing: policy.allow_peer_sharing,
        source_selection_strategy: policy.source_selection_strategy,
        fragment_priority_strategy: policy.fragment_priority_strategy,
        failure_threshold: policy.failure_threshold,
        fallback_supports_range: policy.fallback_mode != "DISABLED",
        fallback_mode: policy.fallback_mode,
        preserve_validated_fragments: true,
        updated_at: policy.updated_at,
    })
}

async fn revalidate_access_package_inner(
    state: &AppState,
    headers: &HeaderMap,
    package_id: &str,
    bucket_name: &str,
    object_key: &str,
    package_token: &str,
) -> anyhow::Result<RevalidateAccessPackageResponse> {
    let authorization = state
        .catalog
        .authorize_access_package(
            package_id,
            &hash_bearer_token(package_token),
            bucket_name,
            object_key,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("access package is invalid, expired or revoked"))?;
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let base_url = request_s3_base_url(headers);
    let object_endpoint = format!(
        "{base_url}/{}/{}",
        url_component(bucket_name),
        object_path(object_key)
    );
    let expires_at = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
    let authorized_sources = authorized_sources_for_object(
        state,
        bucket_name,
        object_key,
        &expires_at,
        &object_endpoint,
        &policy,
    )
    .await?;
    let revalidate_endpoint = Some(format!(
        "{}/pontemesh/access-packages/{}/revalidate/{}/{}",
        request_base_url(headers).trim_end_matches('/'),
        url_component(package_id),
        url_component(bucket_name),
        object_path(object_key)
    ));

    record_mesh_audit(
        state,
        "access_package_revalidated",
        &authorization.application_id,
        "success",
        &format!(
            "package_id={}; manifest_id={}; bucket={}; key={}",
            authorization.package_id,
            authorization.manifest_id,
            authorization.bucket_name,
            authorization.object_key
        ),
    )
    .await;

    Ok(RevalidateAccessPackageResponse {
        package_id: authorization.package_id,
        bucket: authorization.bucket_name,
        key: authorization.object_key,
        manifest_id: authorization.manifest_id,
        valid: true,
        authorized_sources,
        source_selection: source_selection_contract(&policy),
        fallback: fallback_contract(&object_endpoint, &policy, revalidate_endpoint),
    })
}

async fn announce_peer_availability_inner(
    state: &AppState,
    package_id: &str,
    bucket_name: &str,
    object_key: &str,
    package_token: &str,
    payload: AnnouncePeerRequest,
) -> anyhow::Result<PeerAvailabilityRecord> {
    let authorization = state
        .catalog
        .authorize_access_package(
            package_id,
            &hash_bearer_token(package_token),
            bucket_name,
            object_key,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("access package is invalid, expired or revoked"))?;
    let record = state
        .catalog
        .record_peer_fragment_availability(
            &authorization,
            PeerAvailabilityInput {
                peer_id: payload.peer_id,
                endpoint: payload.endpoint,
                available_fragments: payload.available_fragments,
                ttl_seconds: payload.ttl_seconds.unwrap_or(300),
            },
        )
        .await?;
    record_mesh_audit(
        state,
        "peer_availability_announced",
        &authorization.application_id,
        "success",
        &format!(
            "package_id={}; peer_id={}; manifest_id={}; bucket={}; key={}",
            authorization.package_id,
            record.peer_id,
            authorization.manifest_id,
            authorization.bucket_name,
            authorization.object_key
        ),
    )
    .await;
    Ok(record)
}

async fn record_sdk_fragment_event_inner(
    state: &AppState,
    package_id: &str,
    bucket_name: &str,
    object_key: &str,
    package_token: &str,
    payload: SdkFragmentEventRequest,
) -> anyhow::Result<SdkFragmentEventRecord> {
    let authorization = state
        .catalog
        .authorize_access_package(
            package_id,
            &hash_bearer_token(package_token),
            bucket_name,
            object_key,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("access package is invalid, expired or revoked"))?;
    let record = state
        .catalog
        .record_sdk_fragment_event(
            &authorization,
            SdkFragmentEventInput {
                source_type: payload.source_type,
                peer_availability_id: payload.peer_availability_id,
                fragment_index: payload.fragment_index,
                fragment_hash: payload.fragment_hash,
                event_type: payload.event_type,
                bytes_transferred: payload.bytes_transferred,
                outcome: payload.outcome,
                latency_ms: payload.latency_ms,
                detail: payload.detail.unwrap_or_else(|| serde_json::json!({})),
            },
        )
        .await?;
    record_mesh_audit(
        state,
        "sdk_fragment_event_recorded",
        &authorization.application_id,
        "success",
        &format!(
            "package_id={}; manifest_id={}; fragment_index={}; source_type={}; event_type={}; outcome={}",
            authorization.package_id,
            authorization.manifest_id,
            record.fragment_index,
            record.source_type,
            record.event_type,
            record.outcome
        ),
    )
    .await;
    Ok(record)
}

async fn authorized_sources_for_object(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
    expires_at: &str,
    origin_endpoint: &str,
    policy: &BucketPolicy,
) -> anyhow::Result<Vec<AuthorizedSource>> {
    let origin = AuthorizedSource {
        id: "origin".to_owned(),
        source_type: "ORIGIN".to_owned(),
        endpoint: origin_endpoint.to_owned(),
        priority: 0,
        expires_at: expires_at.to_owned(),
        available_fragments: Vec::new(),
    };

    if policy.source_selection_strategy == "ORIGIN_ONLY" {
        return Ok(with_priorities(vec![origin]));
    }

    let replicas = state
        .catalog
        .list_authorized_replica_sources(bucket_name, object_key)
        .await?;
    let replica_sources = replicas
        .into_iter()
        .map(|replica| replica_source(replica, expires_at))
        .collect::<Vec<_>>();
    let peer_sources = if policy.allow_peer_sharing {
        let peers = state
            .catalog
            .list_authorized_peer_sources(bucket_name, object_key)
            .await?;
        peers.into_iter().map(peer_source).collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut sources = Vec::new();
    match policy.source_selection_strategy.as_str() {
        "REPLICA_EDGE_FIRST" => {
            sources.extend(replica_sources);
            sources.extend(peer_sources);
            sources.push(origin);
        }
        "PEER_FIRST" => {
            sources.extend(peer_sources);
            sources.extend(replica_sources);
            sources.push(origin);
        }
        _ => {
            sources.push(origin);
            sources.extend(replica_sources);
            sources.extend(peer_sources);
        }
    }
    Ok(with_priorities(sources))
}

fn replica_source(replica: ReplicaAvailabilityRecord, expires_at: &str) -> AuthorizedSource {
    AuthorizedSource {
        id: replica.replica_id,
        source_type: "REPLICA_EDGE".to_owned(),
        endpoint: replica.endpoint,
        priority: 0,
        expires_at: expires_at.to_owned(),
        available_fragments: replica.available_fragments,
    }
}

fn peer_source(peer: PeerAvailabilityRecord) -> AuthorizedSource {
    AuthorizedSource {
        id: peer.id,
        source_type: "PEER".to_owned(),
        endpoint: peer.endpoint,
        priority: 0,
        expires_at: peer.expires_at,
        available_fragments: peer.available_fragments,
    }
}

fn with_priorities(mut sources: Vec<AuthorizedSource>) -> Vec<AuthorizedSource> {
    for (index, source) in sources.iter_mut().enumerate() {
        source.priority = u8::try_from(index + 1).unwrap_or(u8::MAX);
    }
    sources
}

fn source_selection_contract(policy: &BucketPolicy) -> SourceSelectionContract {
    SourceSelectionContract {
        strategy: policy.source_selection_strategy.clone(),
        fragment_priority: policy.fragment_priority_strategy.clone(),
        failure_threshold: policy.failure_threshold,
        allow_peer_sharing: policy.allow_peer_sharing,
        allow_replica_edge: policy.allow_replica_edge,
    }
}

fn fallback_contract(
    object_endpoint: &str,
    policy: &BucketPolicy,
    revalidate_endpoint: Option<String>,
) -> FallbackContract {
    FallbackContract {
        source_type: "ORIGIN".to_owned(),
        object_endpoint: object_endpoint.to_owned(),
        supports_range: policy.fallback_mode != "DISABLED",
        preserve_validated_fragments: true,
        mode: policy.fallback_mode.clone(),
        revalidate_endpoint,
    }
}

async fn load_manifest(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
) -> anyhow::Result<ManifestResponse> {
    let manifest = state
        .catalog
        .get_object_manifest(bucket_name, object_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("object not found"))?;
    if manifest.availability_state != "AVAILABLE" {
        bail!("object is not available");
    }

    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    build_manifest_with_policy(bucket_name, object_key, manifest, &policy)
}

async fn load_manifest_with_policy(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
    policy: &BucketPolicy,
) -> anyhow::Result<ManifestResponse> {
    let manifest = state
        .catalog
        .get_object_manifest(bucket_name, object_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("object not found"))?;
    if manifest.availability_state != "AVAILABLE" {
        bail!("object is not available");
    }

    build_manifest_with_policy(bucket_name, object_key, manifest, policy)
}

fn build_manifest_with_policy(
    _bucket_name: &str,
    _object_key: &str,
    manifest: ObjectManifest,
    _policy: &BucketPolicy,
) -> anyhow::Result<ManifestResponse> {
    Ok(ManifestResponse {
        manifest_id: manifest.manifest_id,
        object_id: manifest.object_id,
        bucket: manifest.bucket,
        key: manifest.key,
        version: manifest.version,
        total_size_bytes: manifest.total_size_bytes,
        content_type: manifest.content_type,
        object_hash_algorithm: manifest.object_hash_algorithm,
        object_sha256: manifest.object_sha256,
        fragment_size_bytes: usize::try_from(manifest.fragment_size_bytes)
            .map_err(|_| anyhow::anyhow!("fragment size is too large"))?,
        fragments: manifest
            .fragments
            .into_iter()
            .map(|fragment| {
                let start = u64::try_from(fragment.byte_range_start)
                    .map_err(|_| anyhow::anyhow!("fragment byte range is invalid"))?;
                let end = u64::try_from(fragment.byte_range_end)
                    .map_err(|_| anyhow::anyhow!("fragment byte range is invalid"))?;
                let size = usize::try_from(fragment.size_bytes)
                    .map_err(|_| anyhow::anyhow!("fragment size is too large"))?;
                Ok(FragmentDescriptor {
                    index: usize::try_from(fragment.index)
                        .map_err(|_| anyhow::anyhow!("fragment index is too large"))?,
                    fragment_id: fragment.fragment_id,
                    byte_range_start: start,
                    byte_range_end: end,
                    size_bytes: size,
                    hash_algorithm: fragment.hash_algorithm,
                    sha256: fragment.sha256,
                    priority: fragment.priority,
                    fallback_range_header: format!("bytes={start}-{end}"),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
        availability_state: manifest.availability_state,
        created_at: manifest.created_at,
    })
}

fn request_base_url(headers: &HeaderMap) -> String {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost");
    format!("{proto}://{host}")
}

fn request_s3_base_url(headers: &HeaderMap) -> String {
    if let Ok(endpoint) = std::env::var("PONTEMESH_PUBLIC_S3_ENDPOINT") {
        let endpoint = endpoint.trim_end_matches('/');
        if !endpoint.is_empty() {
            return endpoint.to_owned();
        }
    }

    request_base_url(headers)
        .replace(":8080", ":9000")
        .trim_end_matches('/')
        .to_owned()
}

fn object_path(value: &str) -> String {
    value
        .split('/')
        .map(url_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn url_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn has_scope(application: &ApplicationIdentity, required: &str) -> bool {
    application
        .scopes
        .iter()
        .any(|scope| scope == "*" || scope == required)
}

fn read_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
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
    object: &ObjectRecord,
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
        .expect("valid access package object response")
}

fn bad_request(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message == "object not found" {
        StatusCode::NOT_FOUND
    } else if message == "object is not available" {
        StatusCode::FORBIDDEN
    } else if message == "access package is invalid, expired or revoked" {
        StatusCode::UNAUTHORIZED
    } else if message == "requested range is not satisfiable" {
        StatusCode::RANGE_NOT_SATISFIABLE
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

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
        .into_response()
}

async fn record_mesh_audit(
    state: &AppState,
    event: &str,
    principal: &str,
    outcome: &str,
    detail: &str,
) {
    if let Err(error) = state
        .catalog
        .record_audit_event(event, Some(principal), outcome, detail)
        .await
    {
        audit::failure("audit_persist_failed", Some(principal), &error.to_string());
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
