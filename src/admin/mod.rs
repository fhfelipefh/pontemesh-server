use crate::{
    audit,
    auth::AdminSession,
    catalog::{self, BucketPolicyUpdate, BucketSummary, NewObject, ObjectSummary, ObjectTotals},
    config::{self, InstanceRole},
    http::AppState,
    security::s3_secret::s3_secret_encryption_key,
    system::{environment, resources, storage},
};
use anyhow::Context;
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const DEFAULT_S3_ACCESS_KEYS_PAGE_SIZE: u32 = 10;
const MAX_S3_ACCESS_KEYS_PAGE_SIZE: u32 = 100;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    instance: InstanceSummary,
    storage: storage::StorageStatus,
    objects: ObjectTotals,
    resources: resources::ResourceUsage,
    health: HealthSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub name: String,
    pub role: InstanceRole,
    pub environment: environment::RuntimeEnvironment,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthSummary {
    database_connected: bool,
    storage_writable: bool,
    setup_completed: bool,
    authenticated: bool,
    last_checked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApplicationCredentialRequest {
    name: String,
    scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBucketPolicyRequest {
    access_package_ttl_seconds: i64,
    fragment_size_bytes: i64,
    allow_replica_edge: bool,
    allow_peer_sharing: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReplicaCredentialRequest {
    name: String,
    allowed_buckets: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateS3AccessKeyRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListS3AccessKeysQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn dashboard_summary(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
) -> Response {
    audit::event(
        "dashboard_accessed",
        Some(&session.username),
        "success",
        "dashboard summary requested",
    );

    match build_dashboard_summary(&state).await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn instance_summary(State(state): State<AppState>) -> Response {
    match build_instance_summary(&state) {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn system_resources() -> Response {
    let usage = tokio::task::spawn_blocking(resources::collect)
        .await
        .unwrap_or_else(|error| resources::ResourceUsage {
            cpu_usage_percent: None,
            memory_used_bytes: None,
            memory_total_bytes: None,
            memory_usage_percent: None,
            process_memory_bytes: None,
            source: "unavailable".to_owned(),
            warnings: vec![format!("Resource collection task failed: {error}")],
        });
    Json(usage).into_response()
}

pub async fn storage_status(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
) -> Response {
    match config::configured_storage_dir(&state.paths) {
        Ok(path) => {
            let status = storage::status(&path);
            audit::event(
                "storage_status_checked",
                Some(&session.username),
                "success",
                "storage status requested",
            );
            Json(status).into_response()
        }
        Err(error) => internal_error(error),
    }
}

pub async fn list_audit_events(State(state): State<AppState>) -> Response {
    match state.catalog.list_audit_events(100).await {
        Ok(events) => Json(events).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn origin_traffic_metrics(State(state): State<AppState>) -> Response {
    match state.catalog.origin_traffic_summary().await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn list_buckets(State(state): State<AppState>) -> Response {
    match state.catalog.list_buckets().await {
        Ok(buckets) => Json(buckets).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn create_bucket(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<CreateBucketRequest>,
) -> Response {
    match create_bucket_inner(&state, payload.name.trim()).await {
        Ok(bucket) => {
            audit::event(
                "bucket_created",
                Some(&session.username),
                "success",
                &format!("bucket={}", bucket.name),
            );
            record_admin_audit(
                &state,
                "bucket_created",
                &session.username,
                "success",
                &format!("bucket={}", bucket.name),
            )
            .await;
            (StatusCode::CREATED, Json(bucket)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_bucket(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.get_bucket(&bucket_name).await {
        Ok(Some(bucket)) => Json(bucket).into_response(),
        Ok(None) => not_found("bucket not found"),
        Err(error) => bad_request(error),
    }
}

pub async fn delete_bucket(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.delete_bucket(&bucket_name).await {
        Ok(()) => {
            audit::event(
                "bucket_deleted",
                Some(&session.username),
                "success",
                &format!("bucket={bucket_name}"),
            );
            record_admin_audit(
                &state,
                "bucket_deleted",
                &session.username,
                "success",
                &format!("bucket={bucket_name}"),
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_bucket_policy(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.get_bucket_policy(&bucket_name).await {
        Ok(policy) => Json(policy).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn update_bucket_policy(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(bucket_name): Path<String>,
    Json(payload): Json<UpdateBucketPolicyRequest>,
) -> Response {
    let update = BucketPolicyUpdate {
        access_package_ttl_seconds: payload.access_package_ttl_seconds,
        fragment_size_bytes: payload.fragment_size_bytes,
        allow_replica_edge: payload.allow_replica_edge,
        allow_peer_sharing: payload.allow_peer_sharing,
    };
    match state
        .catalog
        .update_bucket_policy(&bucket_name, update)
        .await
    {
        Ok(policy) => {
            audit::event(
                "bucket_policy_updated",
                Some(&session.username),
                "success",
                &format!("bucket={bucket_name}"),
            );
            record_admin_audit(
                &state,
                "bucket_policy_updated",
                &session.username,
                "success",
                &format!("bucket={bucket_name}"),
            )
            .await;
            Json(policy).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn list_objects(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.list_objects(&bucket_name).await {
        Ok(objects) => Json(objects).into_response(),
        Err(error) => bad_request(error),
    }
}

pub async fn upload_object(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(bucket_name): Path<String>,
    multipart: Multipart,
) -> Response {
    match upload_object_inner(&state, &bucket_name, multipart).await {
        Ok(object) => {
            audit::event(
                "object_uploaded",
                Some(&session.username),
                "success",
                &format!("bucket={bucket_name}; key={}", object.key),
            );
            record_admin_audit(
                &state,
                "object_uploaded",
                &session.username,
                "success",
                &format!("bucket={bucket_name}; key={}", object.key),
            )
            .await;
            (StatusCode::CREATED, Json(object)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn get_object(
    State(state): State<AppState>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    match state.catalog.get_object(&bucket_name, &object_key).await {
        Ok(Some(object)) => Json(object).into_response(),
        Ok(None) => not_found("object not found"),
        Err(error) => bad_request(error),
    }
}

pub async fn delete_object(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    match state.catalog.delete_object(&bucket_name, &object_key).await {
        Ok(()) => {
            audit::event(
                "object_deleted",
                Some(&session.username),
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            );
            record_admin_audit(
                &state,
                "object_deleted",
                &session.username,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_object(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    match state.catalog.revoke_object(&bucket_name, &object_key).await {
        Ok(()) => {
            audit::event(
                "object_revoked",
                Some(&session.username),
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            );
            record_admin_audit(
                &state,
                "object_revoked",
                &session.username,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn list_application_credentials(State(state): State<AppState>) -> Response {
    match state.catalog.list_application_credentials().await {
        Ok(credentials) => Json(credentials).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn create_application_credential(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<CreateApplicationCredentialRequest>,
) -> Response {
    let scopes = payload.scopes.unwrap_or_else(default_application_scopes);
    match state
        .catalog
        .create_application_credential(&payload.name, scopes)
        .await
    {
        Ok(created) => {
            audit::event(
                "application_credential_created",
                Some(&session.username),
                "success",
                &format!("application_id={}", created.credential.id),
            );
            record_admin_audit(
                &state,
                "application_credential_created",
                &session.username,
                "success",
                &format!("application_id={}", created.credential.id),
            )
            .await;
            (StatusCode::CREATED, Json(created)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn list_s3_access_keys(
    State(state): State<AppState>,
    Query(query): Query<ListS3AccessKeysQuery>,
) -> Response {
    let (page, page_size) = normalize_s3_access_key_pagination(&query);

    match state.catalog.list_s3_access_keys(page, page_size).await {
        Ok(keys) => Json(keys).into_response(),
        Err(error) => internal_error(error),
    }
}

fn normalize_s3_access_key_pagination(query: &ListS3AccessKeysQuery) -> (u32, u32) {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_S3_ACCESS_KEYS_PAGE_SIZE)
        .clamp(1, MAX_S3_ACCESS_KEYS_PAGE_SIZE);
    (page, page_size)
}

pub async fn create_s3_access_key(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<CreateS3AccessKeyRequest>,
) -> Response {
    let secret_encryption_key = match s3_secret_encryption_key(&state.paths) {
        Ok(key) => key,
        Err(error) => return internal_error(error),
    };
    match state
        .catalog
        .create_s3_access_key(
            &session.user_id,
            payload.name.as_deref(),
            &secret_encryption_key,
        )
        .await
    {
        Ok(created) => {
            audit::event(
                "s3_access_key_created",
                Some(&session.username),
                "success",
                &format!("access_key_id={}", created.access_key_id),
            );
            record_admin_audit(
                &state,
                "s3_access_key_created",
                &session.username,
                "success",
                &format!("access_key_id={}", created.access_key_id),
            )
            .await;
            (StatusCode::CREATED, Json(created)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_s3_access_key_by_id(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
) -> Response {
    match state.catalog.revoke_s3_access_key_by_id(&id).await {
        Ok(key) => {
            audit::event(
                "s3_access_key_revoked",
                Some(&session.username),
                "success",
                &format!("access_key_id={}", key.access_key_id),
            );
            record_admin_audit(
                &state,
                "s3_access_key_revoked",
                &session.username,
                "success",
                &format!("access_key_id={}", key.access_key_id),
            )
            .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_s3_access_key(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(access_key_id): Path<String>,
) -> Response {
    match state.catalog.revoke_s3_access_key(&access_key_id).await {
        Ok(()) => {
            audit::event(
                "s3_access_key_revoked",
                Some(&session.username),
                "success",
                &format!("access_key_id={access_key_id}"),
            );
            record_admin_audit(
                &state,
                "s3_access_key_revoked",
                &session.username,
                "success",
                &format!("access_key_id={access_key_id}"),
            )
            .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn list_replicas(State(state): State<AppState>) -> Response {
    match state.catalog.list_replicas().await {
        Ok(replicas) => Json(replicas).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn create_replica_credential(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<CreateReplicaCredentialRequest>,
) -> Response {
    match state
        .catalog
        .create_replica_credential(&payload.name, payload.allowed_buckets)
        .await
    {
        Ok(created) => {
            audit::event(
                "replica_credential_created",
                Some(&session.username),
                "success",
                &format!("replica_id={}", created.replica.id),
            );
            record_admin_audit(
                &state,
                "replica_credential_created",
                &session.username,
                "success",
                &format!("replica_id={}", created.replica.id),
            )
            .await;
            (StatusCode::CREATED, Json(created)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_replica(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(replica_id): Path<String>,
) -> Response {
    match state.catalog.revoke_replica(&replica_id).await {
        Ok(()) => {
            audit::event(
                "replica_revoked",
                Some(&session.username),
                "success",
                &format!("replica_id={replica_id}"),
            );
            record_admin_audit(
                &state,
                "replica_revoked",
                &session.username,
                "success",
                &format!("replica_id={replica_id}"),
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(error) => bad_request(error),
    }
}

async fn record_admin_audit(
    state: &AppState,
    event: &str,
    username: &str,
    outcome: &str,
    detail: &str,
) {
    if let Err(error) = state
        .catalog
        .record_audit_event(event, Some(username), outcome, detail)
        .await
    {
        audit::failure("audit_persist_failed", Some(username), &error.to_string());
    }
}

async fn build_dashboard_summary(state: &AppState) -> anyhow::Result<DashboardSummary> {
    let storage_path = config::configured_storage_dir(&state.paths)?;
    let storage_status = storage::status(&storage_path);
    let resources = tokio::task::spawn_blocking(resources::collect)
        .await
        .context("resources task failed")?;
    let objects = state.catalog.totals().await?;
    let database_connected = state.catalog.database_connected().await;
    let storage_writable = storage_status.writable;

    Ok(DashboardSummary {
        instance: build_instance_summary(state)?,
        storage: storage_status,
        objects,
        resources,
        health: HealthSummary {
            database_connected,
            storage_writable,
            setup_completed: state.paths.setup_lock_file().exists(),
            authenticated: true,
            last_checked_at: chrono::Utc::now(),
        },
    })
}

fn build_instance_summary(state: &AppState) -> anyhow::Result<InstanceSummary> {
    let config = config::load_instance_config(&state.paths)?;
    Ok(InstanceSummary {
        name: config.instance.name,
        role: config.instance.role,
        environment: environment::detect_environment(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

async fn create_bucket_inner(state: &AppState, bucket_name: &str) -> anyhow::Result<BucketSummary> {
    catalog::validate_bucket_name(bucket_name)?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    fs::create_dir_all(bucket_storage_dir(storage_path, bucket_name))
        .with_context(|| format!("failed to create storage directory for bucket {bucket_name}"))?;
    state.catalog.create_bucket(bucket_name).await
}

async fn upload_object_inner(
    state: &AppState,
    bucket_name: &str,
    mut multipart: Multipart,
) -> anyhow::Result<ObjectSummary> {
    catalog::validate_bucket_name(bucket_name)?;

    let mut requested_key: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .context("failed to read multipart field")?
    {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "key" => {
                requested_key = Some(field.text().await.context("failed to read object key")?);
            }
            "file" => {
                file_name = field.file_name().map(ToOwned::to_owned);
                content_type = field.content_type().map(ToOwned::to_owned);
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .context("failed to read uploaded file")?,
                );
            }
            _ => {}
        }
    }

    let key = requested_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(file_name)
        .ok_or_else(|| anyhow::anyhow!("upload must include a file or object key"))?;
    catalog::validate_object_key(&key)?;

    let bytes = file_bytes.ok_or_else(|| anyhow::anyhow!("upload must include a file"))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_owned());
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let manifest = catalog::build_object_manifest(&bytes, policy.fragment_size_bytes)?;

    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = bucket_storage_dir(storage_path, bucket_name);
    fs::create_dir_all(&bucket_dir).with_context(|| {
        format!(
            "failed to create bucket storage directory {}",
            bucket_dir.display()
        )
    })?;
    let object_path = bucket_dir.join(format!("{}-{}", uuid::Uuid::new_v4(), sha256));
    fs::write(&object_path, &bytes)
        .with_context(|| format!("failed to write object data {}", object_path.display()))?;

    state
        .catalog
        .insert_object(NewObject {
            bucket_name: bucket_name.to_owned(),
            key,
            size_bytes: i64::try_from(bytes.len()).context("uploaded object is too large")?,
            content_type,
            sha256,
            storage_path: object_path.display().to_string(),
            manifest,
        })
        .await
}

fn bucket_storage_dir(storage_path: PathBuf, bucket_name: &str) -> PathBuf {
    storage_path.join("buckets").join(bucket_name)
}

fn default_application_scopes() -> Vec<String> {
    vec![
        "origin:objects:read".to_owned(),
        "origin:objects:write".to_owned(),
        "pontemesh:access-package:create".to_owned(),
        "pontemesh:manifest:read".to_owned(),
    ]
}

fn bad_request(error: anyhow::Error) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn not_found(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
        .into_response()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_access_key_pagination_defaults_to_first_page() {
        let query = ListS3AccessKeysQuery {
            page: None,
            page_size: None,
        };

        assert_eq!(
            normalize_s3_access_key_pagination(&query),
            (1, DEFAULT_S3_ACCESS_KEYS_PAGE_SIZE)
        );
    }

    #[test]
    fn s3_access_key_pagination_clamps_invalid_bounds() {
        let zero_values = ListS3AccessKeysQuery {
            page: Some(0),
            page_size: Some(0),
        };
        let huge_page_size = ListS3AccessKeysQuery {
            page: Some(3),
            page_size: Some(10_000),
        };

        assert_eq!(normalize_s3_access_key_pagination(&zero_values), (1, 1));
        assert_eq!(
            normalize_s3_access_key_pagination(&huge_page_size),
            (3, MAX_S3_ACCESS_KEYS_PAGE_SIZE)
        );
    }
}
