use crate::{
    audit,
    auth::AdminSession,
    catalog::{self, BucketSummary, NewObject, ObjectSummary, ObjectTotals},
    config::{self, InstanceRole},
    http::AppState,
    system::{environment, resources, storage},
};
use anyhow::Context;
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

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
            Json(serde_json::json!({ "ok": true })).into_response()
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
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(error) => bad_request(error),
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
        })
        .await
}

fn bucket_storage_dir(storage_path: PathBuf, bucket_name: &str) -> PathBuf {
    storage_path.join("buckets").join(bucket_name)
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
