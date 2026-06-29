use crate::{
    audit,
    auth::ApplicationIdentity,
    catalog::{BucketPolicy, ObjectManifest},
    http::AppState,
};
use anyhow::bail;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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
    fallback: FallbackContract,
    manifest: ManifestResponse,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackContract {
    source_type: String,
    object_endpoint: String,
    supports_range: bool,
    preserve_validated_fragments: bool,
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
        authorized_sources: vec![AuthorizedSource {
            id: "origin".to_owned(),
            source_type: "ORIGIN".to_owned(),
            endpoint: object_endpoint.clone(),
            priority: 1,
            expires_at: record.expires_at,
        }],
        fallback: FallbackContract {
            source_type: "ORIGIN".to_owned(),
            object_endpoint,
            supports_range: true,
            preserve_validated_fragments: true,
        },
        manifest,
    })
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

fn bad_request(error: anyhow::Error) -> Response {
    let status = if error.to_string() == "object not found" {
        StatusCode::NOT_FOUND
    } else if error.to_string() == "object is not available" {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::BAD_REQUEST
    };
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
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
