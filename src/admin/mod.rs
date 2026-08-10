use crate::{
    audit,
    auth::AdminSession,
    catalog::{
        self, BucketPolicyDefaultsUpdate, BucketPolicyUpdate, BucketSummary, NewObject,
        ObjectSummary, ObjectTotals,
    },
    config::{self, InstanceRole},
    http::AppState,
    security::s3_secret::s3_secret_encryption_key,
    system::{application_logs, environment, resources, storage},
};
use anyhow::Context;
use axum::{
    Extension, Json,
    body::Body,
    extract::{Multipart, Path, Query, State, multipart::Field},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use tokio::io::AsyncWriteExt;

const DEFAULT_S3_ACCESS_KEYS_PAGE_SIZE: u32 = 10;
const MAX_S3_ACCESS_KEYS_PAGE_SIZE: u32 = 100;
const DEFAULT_STORAGE_PAGE_SIZE: u32 = 20;
const MAX_STORAGE_PAGE_SIZE: u32 = 100;
const DEFAULT_APPLICATION_LOGS_LIMIT: usize = 80;
const MAX_APPLICATION_LOGS_LIMIT: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    instance: InstanceSummary,
    storage: storage::StorageStatus,
    objects: ObjectTotals,
    resources: resources::ResourceUsage,
    health: HealthSummary,
    mcp: catalog::McpStatus,
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

#[derive(Debug, Deserialize)]
pub struct UpdateInstanceRequest {
    name: String,
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
pub struct ListBucketsQuery {
    query: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListObjectsQuery {
    query: Option<String>,
    prefix: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApplicationCredentialRequest {
    name: String,
    scopes: Option<Vec<String>>,
    preset: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBucketPolicyRequest {
    access_package_ttl_seconds: i64,
    fragment_size_bytes: i64,
    allow_replica_edge: bool,
    allow_peer_sharing: bool,
    source_selection_strategy: Option<String>,
    fragment_priority_strategy: Option<String>,
    failure_threshold: Option<i64>,
    fallback_mode: Option<String>,
    s3_list_default_max_keys: Option<i64>,
    s3_list_max_keys_limit: Option<i64>,
    s3_list_allow_delimiter: Option<bool>,
    s3_versioning_enabled: Option<bool>,
    s3_object_tagging_enabled: Option<bool>,
    s3_checksum_algorithm: Option<String>,
    s3_multipart_abort_days: Option<i64>,
    s3_default_encryption_algorithm: Option<String>,
    s3_default_encryption_key_id: Option<String>,
    s3_object_lock_enabled: Option<bool>,
    s3_object_lock_default_mode: Option<String>,
    s3_object_lock_default_retain_days: Option<i64>,
    s3_lifecycle_rules: Option<serde_json::Value>,
    s3_resource_policy: Option<serde_json::Value>,
    s3_event_notifications: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkUpdateBucketPoliciesRequest {
    #[serde(default)]
    all_buckets: bool,
    #[serde(default)]
    bucket_names: Vec<String>,
    policy: BucketPolicyDefaultsUpdate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkUpdateBucketPoliciesResponse {
    updated_buckets: Vec<String>,
    updated_count: usize,
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
pub struct CreateMcpTokenRequest {
    name: String,
    #[serde(default = "default_mcp_token_scopes")]
    scopes: Vec<String>,
}

fn default_mcp_token_scopes() -> Vec<String> {
    vec!["read".to_owned()]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListS3AccessKeysQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventsQuery {
    event: Option<String>,
    principal: Option<String>,
    outcome: Option<String>,
    since: Option<chrono::DateTime<chrono::Utc>>,
    until: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLogsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationBackup {
    pub schema_version: u32,
    pub exported_at: Option<chrono::DateTime<chrono::Utc>>,
    pub mcp_settings: Option<ConfigurationMcpSettings>,
    pub bucket_policies: Vec<catalog::BucketPolicy>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationMcpSettings {
    pub enabled: bool,
    pub endpoint_path: String,
    pub bind_host: Option<String>,
    pub require_auth: bool,
    pub read_tools_enabled: bool,
    pub write_tools_enabled: bool,
    #[serde(default)]
    pub admin_tools_enabled: bool,
    pub expose_resources: bool,
    pub expose_prompts: bool,
    pub allow_localhost_only: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationImportResult {
    applied_mcp_settings: bool,
    applied_bucket_policies: usize,
    skipped_bucket_policies: Vec<String>,
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

pub async fn update_instance(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(request): Json<UpdateInstanceRequest>,
) -> Response {
    match config::update_instance_name(&state.paths, &request.name) {
        Ok(_) => {
            audit::event(
                "instance_name_updated",
                Some(&session.username),
                "success",
                "instance display name updated",
            );
            record_admin_audit(
                &state,
                "instance_name_updated",
                &session.username,
                "success",
                "instance display name updated",
            )
            .await;
            match build_instance_summary(&state) {
                Ok(summary) => Json(summary).into_response(),
                Err(error) => internal_error(error),
            }
        }
        Err(error) => bad_request(error),
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

pub async fn list_audit_events(
    State(state): State<AppState>,
    Query(query): Query<AuditEventsQuery>,
) -> Response {
    let filter = catalog::AuditEventFilter {
        event: query.event.filter(|value| !value.trim().is_empty()),
        principal: query.principal.filter(|value| !value.trim().is_empty()),
        outcome: query.outcome.filter(|value| !value.trim().is_empty()),
        since: query.since,
        until: query.until,
        limit: query.limit.unwrap_or(100),
    };
    match state.catalog.list_audit_events_filtered(filter).await {
        Ok(events) => Json(events).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn application_logs(Query(query): Query<ApplicationLogsQuery>) -> Response {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_APPLICATION_LOGS_LIMIT)
        .clamp(1, MAX_APPLICATION_LOGS_LIMIT);
    Json(application_logs::recent(limit)).into_response()
}

pub async fn export_configuration(State(state): State<AppState>) -> Response {
    let mcp_settings = match state.catalog.get_mcp_settings().await {
        Ok(settings) => Some(ConfigurationMcpSettings {
            enabled: settings.enabled,
            endpoint_path: settings.endpoint_path,
            bind_host: settings.bind_host,
            require_auth: settings.require_auth,
            read_tools_enabled: settings.read_tools_enabled,
            write_tools_enabled: settings.write_tools_enabled,
            admin_tools_enabled: settings.admin_tools_enabled,
            expose_resources: settings.expose_resources,
            expose_prompts: settings.expose_prompts,
            allow_localhost_only: settings.allow_localhost_only,
        }),
        Err(error) => return internal_error(error),
    };
    let bucket_policies = match state.catalog.list_bucket_policies().await {
        Ok(policies) => policies,
        Err(error) => return internal_error(error),
    };
    Json(ConfigurationBackup {
        schema_version: 1,
        exported_at: Some(chrono::Utc::now()),
        mcp_settings,
        bucket_policies,
    })
    .into_response()
}

pub async fn import_configuration(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<ConfigurationBackup>,
) -> Response {
    if payload.schema_version != 1 {
        return bad_request(anyhow::anyhow!("unsupported configuration schemaVersion"));
    }

    let mut applied_mcp_settings = false;
    if let Some(settings) = payload.mcp_settings {
        let update = catalog::McpSettingsUpdate {
            enabled: settings.enabled,
            endpoint_path: settings.endpoint_path,
            bind_host: settings.bind_host,
            require_auth: settings.require_auth,
            read_tools_enabled: settings.read_tools_enabled,
            write_tools_enabled: settings.write_tools_enabled,
            admin_tools_enabled: settings.admin_tools_enabled,
            expose_resources: settings.expose_resources,
            expose_prompts: settings.expose_prompts,
            allow_localhost_only: settings.allow_localhost_only,
        };
        if let Err(error) = state.catalog.update_mcp_settings(update).await {
            return bad_request(error);
        }
        applied_mcp_settings = true;
    }

    let mut applied_bucket_policies = 0_usize;
    let mut skipped_bucket_policies = Vec::new();
    for policy in payload.bucket_policies {
        match state.catalog.get_bucket(&policy.bucket_name).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                skipped_bucket_policies.push(policy.bucket_name);
                continue;
            }
            Err(error) => return bad_request(error),
        }
        let update = BucketPolicyUpdate {
            access_package_ttl_seconds: policy.access_package_ttl_seconds,
            fragment_size_bytes: policy.fragment_size_bytes,
            allow_replica_edge: policy.allow_replica_edge,
            allow_peer_sharing: policy.allow_peer_sharing,
            source_selection_strategy: policy.source_selection_strategy,
            fragment_priority_strategy: policy.fragment_priority_strategy,
            failure_threshold: policy.failure_threshold,
            fallback_mode: policy.fallback_mode,
            s3_list_default_max_keys: policy.s3_list_default_max_keys,
            s3_list_max_keys_limit: policy.s3_list_max_keys_limit,
            s3_list_allow_delimiter: policy.s3_list_allow_delimiter,
            s3_versioning_enabled: policy.s3_versioning_enabled,
            s3_object_tagging_enabled: policy.s3_object_tagging_enabled,
            s3_checksum_algorithm: policy.s3_checksum_algorithm,
            s3_multipart_abort_days: policy.s3_multipart_abort_days,
            s3_default_encryption_algorithm: policy.s3_default_encryption_algorithm,
            s3_default_encryption_key_id: policy.s3_default_encryption_key_id,
            s3_object_lock_enabled: policy.s3_object_lock_enabled,
            s3_object_lock_default_mode: policy.s3_object_lock_default_mode,
            s3_object_lock_default_retain_days: policy.s3_object_lock_default_retain_days,
            s3_lifecycle_rules: policy.s3_lifecycle_rules,
            s3_resource_policy: policy.s3_resource_policy,
            s3_event_notifications: policy.s3_event_notifications,
        };
        if let Err(error) = state
            .catalog
            .update_bucket_policy(&policy.bucket_name, update)
            .await
        {
            return bad_request(error);
        }
        applied_bucket_policies += 1;
    }

    record_admin_audit(
        &state,
        "configuration_imported",
        &session.username,
        "success",
        &format!(
            "mcp={applied_mcp_settings}; bucket_policies={applied_bucket_policies}; skipped={}",
            skipped_bucket_policies.len()
        ),
    )
    .await;

    Json(ConfigurationImportResult {
        applied_mcp_settings,
        applied_bucket_policies,
        skipped_bucket_policies,
    })
    .into_response()
}

pub async fn get_mcp_settings(State(state): State<AppState>) -> Response {
    match state.catalog.get_mcp_settings().await {
        Ok(settings) => Json(settings).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn update_mcp_settings(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<catalog::McpSettingsUpdate>,
) -> Response {
    match state.catalog.update_mcp_settings(payload).await {
        Ok(settings) => {
            let event = if settings.enabled {
                "MCP_ENABLED"
            } else {
                "MCP_DISABLED"
            };
            audit::event(
                event,
                Some(&session.username),
                "success",
                "MCP settings updated",
            );
            Json(settings).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn mcp_status(State(state): State<AppState>) -> Response {
    match state.catalog.mcp_status().await {
        Ok(status) => Json(status).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn list_mcp_tokens(State(state): State<AppState>) -> Response {
    match state.catalog.list_mcp_access_tokens().await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn create_mcp_token(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<CreateMcpTokenRequest>,
) -> Response {
    match state
        .catalog
        .create_mcp_access_token(&payload.name, &payload.scopes, Some(&session.user_id))
        .await
    {
        Ok(token) => {
            audit::event(
                "MCP_TOKEN_CREATED",
                Some(&session.username),
                "success",
                "MCP token created",
            );
            (StatusCode::CREATED, Json(token)).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_mcp_token(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
) -> Response {
    match state.catalog.revoke_mcp_access_token(&id).await {
        Ok(()) => {
            audit::event(
                "MCP_TOKEN_REVOKED",
                Some(&session.username),
                "success",
                "MCP token revoked",
            );
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn mcp_activity(State(state): State<AppState>) -> Response {
    match state.catalog.list_mcp_activity(50).await {
        Ok(activity) => Json(activity).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn origin_traffic_metrics(State(state): State<AppState>) -> Response {
    match state.catalog.origin_traffic_summary().await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn replica_traffic_metrics(State(state): State<AppState>) -> Response {
    match state.catalog.replica_traffic_summary().await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn bucket_traffic_metrics(State(state): State<AppState>) -> Response {
    match state.catalog.bucket_traffic_metrics().await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn object_traffic_metrics(State(state): State<AppState>) -> Response {
    match state.catalog.object_traffic_metrics().await {
        Ok(summary) => Json(summary).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn replica_detail_metrics(
    State(state): State<AppState>,
    Path(replica_id): Path<String>,
) -> Response {
    match state.catalog.replica_detail_metrics(&replica_id).await {
        Ok(Some(summary)) => Json(summary).into_response(),
        Ok(None) => not_found("replica not found"),
        Err(error) => bad_request(error),
    }
}

pub async fn list_buckets(
    State(state): State<AppState>,
    Query(query): Query<ListBucketsQuery>,
) -> Response {
    let (page, page_size) = normalize_storage_pagination(query.page, query.page_size);
    match state
        .catalog
        .list_buckets_page(query.query.as_deref(), page, page_size)
        .await
    {
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

pub async fn get_bucket_policy_defaults(State(state): State<AppState>) -> Response {
    match state.catalog.get_bucket_policy_defaults().await {
        Ok(defaults) => Json(defaults).into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn update_bucket_policy_defaults(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<BucketPolicyDefaultsUpdate>,
) -> Response {
    match state.catalog.update_bucket_policy_defaults(payload).await {
        Ok(defaults) => {
            audit::event(
                "bucket_policy_defaults_updated",
                Some(&session.username),
                "success",
                "instance bucket policy defaults updated",
            );
            record_admin_audit(
                &state,
                "bucket_policy_defaults_updated",
                &session.username,
                "success",
                "instance bucket policy defaults updated",
            )
            .await;
            Json(defaults).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn bulk_update_bucket_policies(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Json(payload): Json<BulkUpdateBucketPoliciesRequest>,
) -> Response {
    let policy_detail =
        serde_json::to_value(&payload.policy).unwrap_or_else(|_| serde_json::json!({}));
    match state
        .catalog
        .bulk_update_bucket_policy(payload.all_buckets, &payload.bucket_names, payload.policy)
        .await
    {
        Ok(updated_buckets) => {
            for bucket_name in &updated_buckets {
                if let Err(error) = state
                    .catalog
                    .record_replica_policy_update_for_bucket(
                        bucket_name,
                        None,
                        "BUCKET_POLICY_UPDATED",
                        policy_detail.clone(),
                    )
                    .await
                {
                    audit::failure(
                        "replica_policy_update_persist_failed",
                        Some(&session.username),
                        &error.to_string(),
                    );
                }
            }
            let detail = format!("updatedBuckets={}", updated_buckets.len());
            audit::event(
                "bucket_policies_bulk_updated",
                Some(&session.username),
                "success",
                &detail,
            );
            record_admin_audit(
                &state,
                "bucket_policies_bulk_updated",
                &session.username,
                "success",
                &detail,
            )
            .await;
            Json(BulkUpdateBucketPoliciesResponse {
                updated_count: updated_buckets.len(),
                updated_buckets,
            })
            .into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn update_bucket_policy(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(bucket_name): Path<String>,
    Json(payload): Json<UpdateBucketPolicyRequest>,
) -> Response {
    let current = match state.catalog.get_bucket_policy(&bucket_name).await {
        Ok(policy) => policy,
        Err(error) => return bad_request(error),
    };
    let update = BucketPolicyUpdate {
        access_package_ttl_seconds: payload.access_package_ttl_seconds,
        fragment_size_bytes: payload.fragment_size_bytes,
        allow_replica_edge: payload.allow_replica_edge,
        allow_peer_sharing: payload.allow_peer_sharing,
        source_selection_strategy: payload
            .source_selection_strategy
            .unwrap_or_else(|| "ORIGIN_REPLICA_EDGE".to_owned()),
        fragment_priority_strategy: payload
            .fragment_priority_strategy
            .unwrap_or_else(|| "MANIFEST_ORDER".to_owned()),
        failure_threshold: payload.failure_threshold.unwrap_or(3),
        fallback_mode: payload
            .fallback_mode
            .unwrap_or_else(|| "ORIGIN_RANGE".to_owned()),
        s3_list_default_max_keys: payload.s3_list_default_max_keys.unwrap_or(1000),
        s3_list_max_keys_limit: payload.s3_list_max_keys_limit.unwrap_or(10_000),
        s3_list_allow_delimiter: payload.s3_list_allow_delimiter.unwrap_or(true),
        s3_versioning_enabled: payload.s3_versioning_enabled.unwrap_or(false),
        s3_object_tagging_enabled: payload.s3_object_tagging_enabled.unwrap_or(true),
        s3_checksum_algorithm: payload
            .s3_checksum_algorithm
            .unwrap_or_else(|| "SHA256".to_owned()),
        s3_multipart_abort_days: payload.s3_multipart_abort_days.unwrap_or(7),
        s3_default_encryption_algorithm: payload
            .s3_default_encryption_algorithm
            .unwrap_or(current.s3_default_encryption_algorithm),
        s3_default_encryption_key_id: payload
            .s3_default_encryption_key_id
            .or(current.s3_default_encryption_key_id),
        s3_object_lock_enabled: payload
            .s3_object_lock_enabled
            .unwrap_or(current.s3_object_lock_enabled),
        s3_object_lock_default_mode: payload
            .s3_object_lock_default_mode
            .or(current.s3_object_lock_default_mode),
        s3_object_lock_default_retain_days: payload
            .s3_object_lock_default_retain_days
            .or(current.s3_object_lock_default_retain_days),
        s3_lifecycle_rules: payload
            .s3_lifecycle_rules
            .unwrap_or(current.s3_lifecycle_rules),
        s3_resource_policy: payload
            .s3_resource_policy
            .unwrap_or(current.s3_resource_policy),
        s3_event_notifications: payload
            .s3_event_notifications
            .unwrap_or(current.s3_event_notifications),
    };
    match state
        .catalog
        .update_bucket_policy(&bucket_name, update)
        .await
    {
        Ok(policy) => {
            let update_detail = serde_json::json!({
                "accessPackageTtlSeconds": policy.access_package_ttl_seconds,
                "fragmentSizeBytes": policy.fragment_size_bytes,
                "allowReplicaEdge": policy.allow_replica_edge,
                "allowPeerSharing": policy.allow_peer_sharing,
                "sourceSelectionStrategy": policy.source_selection_strategy,
                "fragmentPriorityStrategy": policy.fragment_priority_strategy,
                "failureThreshold": policy.failure_threshold,
                "fallbackMode": policy.fallback_mode,
                "s3ListDefaultMaxKeys": policy.s3_list_default_max_keys,
                "s3ListMaxKeysLimit": policy.s3_list_max_keys_limit,
                "s3ListAllowDelimiter": policy.s3_list_allow_delimiter,
                "s3VersioningEnabled": policy.s3_versioning_enabled,
                "s3ObjectTaggingEnabled": policy.s3_object_tagging_enabled,
                "s3ChecksumAlgorithm": policy.s3_checksum_algorithm,
                "s3MultipartAbortDays": policy.s3_multipart_abort_days
            });
            if let Err(error) = state
                .catalog
                .record_replica_policy_update_for_bucket(
                    &bucket_name,
                    None,
                    "BUCKET_POLICY_UPDATED",
                    update_detail,
                )
                .await
            {
                audit::failure(
                    "replica_policy_update_persist_failed",
                    Some(&session.username),
                    &error.to_string(),
                );
            }
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
    Query(query): Query<ListObjectsQuery>,
) -> Response {
    let (page, page_size) = normalize_storage_pagination(query.page, query.page_size);
    match state
        .catalog
        .list_objects_page(&bucket_name, query.query.as_deref(), query.prefix.as_deref(), page, page_size)
        .await
    {
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
            let detail = format!(
                "user={} bucket={bucket_name} key={} size_bytes={}",
                session.username, object.key, object.size_bytes
            );
            tracing::info!(target: "pontemesh_admin", "{detail}");
            application_logs::info("admin.upload", detail);
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
        Err(error) => {
            let detail = format!(
                "user={} bucket={bucket_name} upload_failed: {error:#}",
                session.username
            );
            tracing::error!(target: "pontemesh_admin", "{detail}");
            application_logs::error("admin.upload", detail.clone());
            record_admin_audit(
                &state,
                "object_upload_failed",
                &session.username,
                "failure",
                &format!("bucket={bucket_name}; error={error:#}"),
            )
            .await;
            bad_request(anyhow::anyhow!(format!("{error:#}")))
        }
    }
}

pub async fn get_object(
    State(state): State<AppState>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    match state
        .catalog
        .get_object_record(&bucket_name, &object_key)
        .await
    {
        Ok(Some(object)) if object.state == "AVAILABLE" => match fs::read(&object.storage_path) {
            Ok(bytes) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, object.content_type.as_str())
                .header(header::CONTENT_LENGTH, bytes.len().to_string())
                .header(
                    header::CONTENT_DISPOSITION,
                    format!(
                        "attachment; filename=\"{}\"",
                        download_filename(&object.key)
                    ),
                )
                .body(Body::from(bytes))
                .expect("valid object download response"),
            Err(error) => internal_error(anyhow::Error::new(error)),
        },
        Ok(Some(_)) => bad_request(anyhow::anyhow!("object is not available")),
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
            if let Err(error) = state
                .catalog
                .record_replica_policy_update_for_bucket(
                    &bucket_name,
                    Some(&object_key),
                    "OBJECT_REVOKED",
                    serde_json::json!({ "bucket": bucket_name, "key": object_key }),
                )
                .await
            {
                audit::failure(
                    "replica_policy_update_persist_failed",
                    Some(&session.username),
                    &error.to_string(),
                );
            }
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
    let scopes = match resolve_application_scopes(payload.scopes, payload.preset.as_deref()) {
        Ok(scopes) => scopes,
        Err(error) => return bad_request(error),
    };
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

pub async fn revoke_application_credential(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(id): Path<String>,
) -> Response {
    match state.catalog.revoke_application_credential(&id).await {
        Ok(()) => {
            audit::event(
                "application_credential_revoked",
                Some(&session.username),
                "success",
                &format!("application_id={id}"),
            );
            record_admin_audit(
                &state,
                "application_credential_revoked",
                &session.username,
                "success",
                &format!("application_id={id}"),
            )
            .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn revoke_access_package(
    State(state): State<AppState>,
    Extension(session): Extension<AdminSession>,
    Path(package_id): Path<String>,
) -> Response {
    match state.catalog.revoke_access_package(&package_id).await {
        Ok(()) => {
            audit::event(
                "access_package_revoked",
                Some(&session.username),
                "success",
                &format!("package_id={package_id}"),
            );
            record_admin_audit(
                &state,
                "access_package_revoked",
                &session.username,
                "success",
                &format!("package_id={package_id}"),
            )
            .await;
            StatusCode::NO_CONTENT.into_response()
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

fn normalize_storage_pagination(page: Option<u32>, page_size: Option<u32>) -> (u32, u32) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size
        .unwrap_or(DEFAULT_STORAGE_PAGE_SIZE)
        .clamp(1, MAX_STORAGE_PAGE_SIZE);
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
    let mcp = state.catalog.mcp_status().await?;
    let database_connected = state.catalog.database_connected().await;
    let storage_writable = storage_status.writable;

    Ok(DashboardSummary {
        instance: build_instance_summary(state)?,
        storage: storage_status,
        objects,
        resources,
        mcp,
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
    let mut uploaded_file: Option<UploadedObjectFile> = None;
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = bucket_storage_dir(storage_path, bucket_name);
    fs::create_dir_all(&bucket_dir).with_context(|| {
        format!(
            "failed to create bucket storage directory {}",
            bucket_dir.display()
        )
    })?;

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
                uploaded_file = Some(
                    persist_uploaded_file(field, &bucket_dir, policy.fragment_size_bytes).await?,
                );
            }
            _ => {}
        }
    }

    let uploaded_file =
        uploaded_file.ok_or_else(|| anyhow::anyhow!("upload must include a file"))?;
    let key = requested_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(uploaded_file.file_name)
        .ok_or_else(|| anyhow::anyhow!("upload must include a file or object key"))?;
    catalog::validate_object_key(&key)?;

    let object_path = bucket_dir.join(format!("{}-{}", uuid::Uuid::new_v4(), uploaded_file.sha256));
    tokio::fs::rename(&uploaded_file.path, &object_path)
        .await
        .with_context(|| format!("failed to finalize object data {}", object_path.display()))?;

    state
        .catalog
        .insert_object(NewObject {
            bucket_name: bucket_name.to_owned(),
            key,
            size_bytes: uploaded_file.size_bytes,
            content_type: uploaded_file.content_type,
            sha256: uploaded_file.sha256,
            storage_path: object_path.display().to_string(),
            checksum_sha256: None,
            checksum_crc32: None,
            encryption_algorithm: None,
            encryption_key_id: None,
            encryption_nonce: None,
            object_lock_mode: None,
            retain_until: None,
            legal_hold: false,
            manifest: uploaded_file.manifest,
            user_metadata: None,
            created_by: None,
        })
        .await
}

struct UploadedObjectFile {
    file_name: Option<String>,
    content_type: String,
    path: PathBuf,
    size_bytes: i64,
    sha256: String,
    manifest: catalog::NewObjectManifest,
}

async fn persist_uploaded_file(
    mut field: Field<'_>,
    bucket_dir: &std::path::Path,
    fragment_size_bytes: i64,
) -> anyhow::Result<UploadedObjectFile> {
    let file_name = field.file_name().map(ToOwned::to_owned);
    let content_type = field
        .content_type()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let temp_path = bucket_dir.join(format!(".upload-{}.part", uuid::Uuid::new_v4()));
    let mut output = tokio::fs::File::create(&temp_path)
        .await
        .with_context(|| format!("failed to create upload file {}", temp_path.display()))?;
    let mut object_hasher = Sha256::new();
    let mut fragment_hasher = Sha256::new();
    let mut fragments = Vec::new();
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    if fragment_size == 0 {
        anyhow::bail!("fragmentSizeBytes must be positive");
    }
    let mut current_fragment_size: usize = 0;
    let mut current_fragment_start: i64 = 0;
    let mut total_size: i64 = 0;

    while let Some(chunk) = field
        .chunk()
        .await
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp_path);
        })
        .context("failed to read uploaded file")?
    {
        output
            .write_all(&chunk)
            .await
            .inspect_err(|_| {
                let _ = std::fs::remove_file(&temp_path);
            })
            .with_context(|| format!("failed to write uploaded file {}", temp_path.display()))?;
        object_hasher.update(&chunk);

        let mut offset = 0;
        while offset < chunk.len() {
            let remaining_fragment = fragment_size - current_fragment_size;
            let take = remaining_fragment.min(chunk.len() - offset);
            fragment_hasher.update(&chunk[offset..offset + take]);
            current_fragment_size += take;
            offset += take;
            total_size = total_size
                .checked_add(i64::try_from(take).context("uploaded object is too large")?)
                .ok_or_else(|| anyhow::anyhow!("uploaded object is too large"))?;

            if current_fragment_size == fragment_size {
                push_fragment(
                    &mut fragments,
                    &mut fragment_hasher,
                    current_fragment_start,
                    current_fragment_size,
                )?;
                current_fragment_start = total_size;
                current_fragment_size = 0;
            }
        }
    }

    if current_fragment_size > 0 {
        push_fragment(
            &mut fragments,
            &mut fragment_hasher,
            current_fragment_start,
            current_fragment_size,
        )?;
    }

    output
        .flush()
        .await
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp_path);
        })
        .with_context(|| format!("failed to flush uploaded file {}", temp_path.display()))?;

    Ok(UploadedObjectFile {
        file_name,
        content_type,
        path: temp_path,
        size_bytes: total_size,
        sha256: format!("{:x}", object_hasher.finalize()),
        manifest: catalog::NewObjectManifest {
            fragment_size_bytes,
            fragments,
        },
    })
}

fn push_fragment(
    fragments: &mut Vec<catalog::NewObjectFragment>,
    fragment_hasher: &mut Sha256,
    start: i64,
    size: usize,
) -> anyhow::Result<()> {
    let size_bytes = i64::try_from(size).context("fragment size cannot fit in i64")?;
    let sha256 = format!("{:x}", std::mem::take(fragment_hasher).finalize());
    fragments.push(catalog::NewObjectFragment {
        index: i64::try_from(fragments.len()).context("fragment index cannot fit in i64")?,
        byte_range_start: start,
        byte_range_end: start + size_bytes.saturating_sub(1),
        size_bytes,
        sha256,
        priority: if fragments.is_empty() {
            "INITIAL".to_owned()
        } else {
            "NORMAL".to_owned()
        },
    });
    Ok(())
}

fn bucket_storage_dir(storage_path: PathBuf, bucket_name: &str) -> PathBuf {
    storage_path.join("buckets").join(bucket_name)
}

fn download_filename(object_key: &str) -> String {
    object_key
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("object")
        .chars()
        .map(|ch| match ch {
            '"' | '\\' | '\r' | '\n' => '_',
            _ => ch,
        })
        .collect()
}

fn default_application_scopes() -> Vec<String> {
    vec![
        "origin:objects:read".to_owned(),
        "origin:objects:write".to_owned(),
        "pontemesh:access-package:create".to_owned(),
        "pontemesh:manifest:read".to_owned(),
        "pontemesh:sources:read".to_owned(),
        "pontemesh:availability:read".to_owned(),
        "pontemesh:policies:read".to_owned(),
    ]
}

fn downloader_application_scopes() -> Vec<String> {
    default_application_scopes()
        .into_iter()
        .filter(|scope| scope != "origin:objects:write")
        .collect()
}

fn resolve_application_scopes(
    scopes: Option<Vec<String>>,
    preset: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    if let Some(scopes) = scopes {
        return Ok(scopes);
    }
    match preset.unwrap_or("downloader") {
        "downloader" => Ok(downloader_application_scopes()),
        "full" => Ok(default_application_scopes()),
        value => anyhow::bail!("unsupported application credential preset: {value}"),
    }
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

    #[test]
    fn downloader_preset_never_grants_object_write() {
        let scopes = resolve_application_scopes(None, Some("downloader")).expect("scopes");

        assert!(scopes.iter().any(|scope| scope == "origin:objects:read"));
        assert!(
            scopes
                .iter()
                .any(|scope| scope == "pontemesh:access-package:create")
        );
        assert!(!scopes.iter().any(|scope| scope == "origin:objects:write"));
    }

    #[test]
    fn unsupported_application_preset_is_rejected() {
        assert!(resolve_application_scopes(None, Some("anonymous-admin")).is_err());
    }
}
