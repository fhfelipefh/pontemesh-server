use crate::{
    config,
    security::{random::secure_url_token, token::hash_bearer_token},
};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use sha2::{Digest, Sha256};
use sqlx::{PgPool, PgPoolOptions, PgRow, Postgres};
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row, transaction::Transaction};
use std::{net::IpAddr, path::Path};

#[derive(Debug, Clone)]
pub struct Catalog {
    pool: PgPool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketSummary {
    pub name: String,
    pub object_count: i64,
    pub total_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedBuckets {
    pub items: Vec<BucketSummary>,
    pub page: u32,
    pub page_size: u32,
    pub total_items: i64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSummary {
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub version_id: Option<String>,
    pub is_delete_marker: bool,
    pub created_at: String,
    pub updated_at: String,
    pub state: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedObjects {
    pub items: Vec<ObjectSummary>,
    pub common_prefixes: Vec<String>,
    pub page: u32,
    pub page_size: u32,
    pub total_items: i64,
    pub total_pages: u32,
}

#[derive(Debug, Clone)]
pub struct ListObjectsV2Options {
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    pub max_keys: i64,
    pub continuation_token: Option<String>,
    pub start_after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct S3ListObjectsPage {
    pub items: Vec<ObjectSummary>,
    pub common_prefixes: Vec<String>,
    pub key_count: usize,
    pub max_keys: i64,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
    pub start_after: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRecord {
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub storage_path: String,
    pub version_id: String,
    pub is_delete_marker: bool,
    pub checksum_sha256: Option<String>,
    pub checksum_crc32: Option<String>,
    pub encryption_algorithm: Option<String>,
    pub encryption_key_id: Option<String>,
    pub encryption_nonce: Option<Vec<u8>>,
    pub object_lock_mode: Option<String>,
    pub retain_until: Option<String>,
    pub legal_hold: bool,
    pub created_at: String,
    pub state: String,
    pub user_metadata: Option<serde_json::Value>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3ObjectTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTotals {
    pub total_buckets: i64,
    pub total_objects: i64,
    pub total_object_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketPolicy {
    pub bucket_name: String,
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
    pub source_selection_strategy: String,
    pub fragment_priority_strategy: String,
    pub failure_threshold: i64,
    pub fallback_mode: String,
    pub s3_list_default_max_keys: i64,
    pub s3_list_max_keys_limit: i64,
    pub s3_list_allow_delimiter: bool,
    pub s3_versioning_enabled: bool,
    pub s3_object_tagging_enabled: bool,
    pub s3_checksum_algorithm: String,
    pub s3_multipart_abort_days: i64,
    pub s3_default_encryption_algorithm: String,
    pub s3_default_encryption_key_id: Option<String>,
    pub s3_object_lock_enabled: bool,
    pub s3_object_lock_default_mode: Option<String>,
    pub s3_object_lock_default_retain_days: Option<i64>,
    pub s3_lifecycle_rules: serde_json::Value,
    pub s3_resource_policy: serde_json::Value,
    pub s3_event_notifications: serde_json::Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketPolicyUpdate {
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
    pub source_selection_strategy: String,
    pub fragment_priority_strategy: String,
    pub failure_threshold: i64,
    pub fallback_mode: String,
    pub s3_list_default_max_keys: i64,
    pub s3_list_max_keys_limit: i64,
    pub s3_list_allow_delimiter: bool,
    pub s3_versioning_enabled: bool,
    pub s3_object_tagging_enabled: bool,
    pub s3_checksum_algorithm: String,
    pub s3_multipart_abort_days: i64,
    pub s3_default_encryption_algorithm: String,
    pub s3_default_encryption_key_id: Option<String>,
    pub s3_object_lock_enabled: bool,
    pub s3_object_lock_default_mode: Option<String>,
    pub s3_object_lock_default_retain_days: Option<i64>,
    pub s3_lifecycle_rules: serde_json::Value,
    pub s3_resource_policy: serde_json::Value,
    pub s3_event_notifications: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketPolicyDefaults {
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
    pub source_selection_strategy: String,
    pub fragment_priority_strategy: String,
    pub failure_threshold: i64,
    pub fallback_mode: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketPolicyDefaultsUpdate {
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
    pub source_selection_strategy: String,
    pub fragment_priority_strategy: String,
    pub failure_threshold: i64,
    pub fallback_mode: String,
}

#[derive(Debug, Clone)]
pub struct NewObject {
    pub bucket_name: String,
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub storage_path: String,
    pub checksum_sha256: Option<String>,
    pub checksum_crc32: Option<String>,
    pub encryption_algorithm: Option<String>,
    pub encryption_key_id: Option<String>,
    pub encryption_nonce: Option<Vec<u8>>,
    pub object_lock_mode: Option<String>,
    pub retain_until: Option<chrono::DateTime<chrono::Utc>>,
    pub legal_hold: bool,
    pub manifest: NewObjectManifest,
    pub user_metadata: Option<serde_json::Value>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectVersionSummary {
    pub key: String,
    pub version_id: String,
    pub is_latest: bool,
    pub is_delete_marker: bool,
    pub size_bytes: i64,
    pub sha256: String,
    pub last_modified: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct S3LifecycleResult {
    pub expired_objects: i64,
    pub aborted_multipart_uploads: i64,
}

#[derive(Debug, Clone)]
pub struct NewObjectManifest {
    pub fragment_size_bytes: i64,
    pub fragments: Vec<NewObjectFragment>,
}

#[derive(Debug, Clone)]
pub struct NewObjectFragment {
    pub index: i64,
    pub byte_range_start: i64,
    pub byte_range_end: i64,
    pub size_bytes: i64,
    pub sha256: String,
    pub priority: String,
}

struct ObjectAuditEvent<'a> {
    event: &'a str,
    principal: &'a str,
    outcome: &'a str,
    detail: &'a str,
}

#[derive(Debug, Clone)]
pub struct ObjectManifest {
    pub manifest_id: String,
    pub object_id: String,
    pub bucket: String,
    pub key: String,
    pub version: String,
    pub total_size_bytes: i64,
    pub content_type: String,
    pub object_hash_algorithm: String,
    pub object_sha256: String,
    pub fragment_size_bytes: i64,
    pub availability_state: String,
    pub created_at: String,
    pub signature_algorithm: Option<String>,
    pub signature: Option<String>,
    pub fragments: Vec<ObjectManifestFragment>,
}

#[derive(Debug, Clone)]
pub struct ObjectManifestFragment {
    pub index: i64,
    pub fragment_id: String,
    pub byte_range_start: i64,
    pub byte_range_end: i64,
    pub size_bytes: i64,
    pub hash_algorithm: String,
    pub sha256: String,
    pub priority: String,
}

#[derive(Debug, Clone)]
pub struct MultipartUploadRecord {
    pub upload_id: String,
    pub bucket_name: String,
    pub object_key: String,
    pub content_type: String,
    pub initiated_at: String,
    pub user_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct MultipartPartRecord {
    pub part_number: i32,
    pub etag: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub uploaded_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationCredentialSummary {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub revoked: bool,
}

#[derive(Debug, Clone)]
pub struct ApplicationCredential {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct S3AccessKey {
    pub access_key_id: String,
    pub secret_key_hash: String,
    pub secret_access_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3AccessKeySummary {
    pub id: String,
    pub name: Option<String>,
    pub access_key_id: String,
    pub user_id: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedS3AccessKeys {
    pub items: Vec<S3AccessKeySummary>,
    pub page: u32,
    pub page_size: u32,
    pub total: i64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedS3AccessKey {
    pub id: String,
    pub name: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedApplicationCredential {
    pub credential: ApplicationCredentialSummary,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaSummary {
    pub id: String,
    pub name: String,
    pub allowed_buckets: Vec<String>,
    pub created_at: String,
    pub revoked: bool,
    pub available_objects: i64,
    pub last_seen_at: Option<String>,
    pub health_status: Option<String>,
    pub health_reported_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplicaCredential {
    pub id: String,
    pub name: String,
    pub allowed_buckets: Vec<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone)]
pub struct AccessPackageAuthorization {
    pub package_id: String,
    pub application_id: String,
    pub bucket_name: String,
    pub object_key: String,
    pub manifest_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedReplicaCredential {
    pub replica: ReplicaSummary,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaSyncObject {
    pub bucket: String,
    pub key: String,
    pub manifest_id: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub state: String,
    pub election_epoch: String,
    pub election_leader_id: Option<String>,
    pub replica_set: Vec<ReplicaSyncMember>,
    pub fragments: Vec<ReplicaSyncFragment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaSyncMember {
    pub replica_id: String,
    pub replica_name: String,
    pub endpoint: Option<String>,
    pub last_seen_at: Option<String>,
}

fn elected_replica_leader(replica_set: &[ReplicaSyncMember]) -> Option<String> {
    replica_set
        .iter()
        .map(|member| member.replica_id.as_str())
        .min()
        .map(str::to_owned)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaSyncFragment {
    pub index: i64,
    pub fragment_id: String,
    pub byte_range_start: i64,
    pub byte_range_end: i64,
    pub size_bytes: i64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaAvailabilityRecord {
    pub replica_id: String,
    pub replica_name: String,
    pub bucket: String,
    pub key: String,
    pub endpoint: String,
    pub available_fragments: Vec<i64>,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerAvailabilityRecord {
    pub id: String,
    pub peer_id: String,
    pub bucket: String,
    pub key: String,
    pub endpoint: String,
    pub available_fragments: Vec<i64>,
    pub expires_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone)]
pub struct PeerAvailabilityInput {
    pub peer_id: String,
    pub endpoint: String,
    pub available_fragments: Vec<i64>,
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct SdkFragmentEventInput {
    pub source_type: String,
    pub peer_availability_id: Option<String>,
    pub fragment_index: i64,
    pub fragment_hash: String,
    pub event_type: String,
    pub bytes_transferred: i64,
    pub outcome: String,
    pub latency_ms: Option<i64>,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkFragmentEventRecord {
    pub id: String,
    pub source_type: String,
    pub fragment_index: i64,
    pub fragment_hash: String,
    pub event_type: String,
    pub bytes_transferred: i64,
    pub outcome: String,
    pub latency_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ReplicaHealthReportInput {
    pub status: String,
    pub version: Option<String>,
    pub storage_available_bytes: Option<i64>,
    pub error_count: i64,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaHealthReport {
    pub replica_id: String,
    pub status: String,
    pub version: Option<String>,
    pub storage_available_bytes: Option<i64>,
    pub error_count: i64,
    pub reported_at: String,
}

#[derive(Debug, Clone)]
pub struct ReplicaMetricInput {
    pub bytes_synced: i64,
    pub bytes_served: i64,
    pub fragments_synced: i64,
    pub fragments_served: i64,
    pub sync_failures: i64,
    pub auth_failures: i64,
    pub avg_latency_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaMetricRecord {
    pub replica_id: String,
    pub bytes_synced: i64,
    pub bytes_served: i64,
    pub fragments_synced: i64,
    pub fragments_served: i64,
    pub sync_failures: i64,
    pub auth_failures: i64,
    pub avg_latency_ms: Option<i64>,
    pub reported_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaTrafficSummary {
    pub total_replicas: i64,
    pub active_replicas: i64,
    pub total_bytes_synced: i64,
    pub total_bytes_served: i64,
    pub total_fragments_synced: i64,
    pub total_fragments_served: i64,
    pub sync_failures: i64,
    pub auth_failures: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaPolicyUpdateRecord {
    pub id: String,
    pub update_type: String,
    pub bucket: Option<String>,
    pub object_key: Option<String>,
    pub detail: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AuthorizedReplicaFragment {
    pub object: ObjectRecord,
    pub bucket_name: String,
    pub object_key: String,
    pub manifest_id: String,
    pub fragment_index: i64,
    pub fragment_hash: String,
    pub byte_range_start: i64,
    pub byte_range_end: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketTrafficMetric {
    pub bucket: String,
    pub origin_bytes_served: i64,
    pub origin_requests: i64,
    pub replica_bytes_synced: i64,
    pub peer_bytes_served: i64,
    pub fragment_events: i64,
    pub fallback_events: i64,
    pub integrity_failures: i64,
    pub origin_offload_bytes: i64,
    pub source_attempts: i64,
    pub fallback_rate: f64,
    pub integrity_failure_rate: f64,
    pub avg_auxiliary_latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTrafficMetric {
    pub bucket: String,
    pub key: String,
    pub origin_bytes_served: i64,
    pub origin_requests: i64,
    pub replica_bytes_synced: i64,
    pub peer_bytes_served: i64,
    pub fragment_events: i64,
    pub fallback_events: i64,
    pub integrity_failures: i64,
    pub origin_offload_bytes: i64,
    pub source_attempts: i64,
    pub fallback_rate: f64,
    pub integrity_failure_rate: f64,
    pub avg_auxiliary_latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicaDetailMetric {
    pub replica_id: String,
    pub replica_name: String,
    pub bytes_synced: i64,
    pub bytes_served: i64,
    pub fragments_synced: i64,
    pub fragments_served: i64,
    pub sync_failures: i64,
    pub auth_failures: i64,
    pub fragment_events: i64,
}

#[derive(Debug, Clone)]
pub struct AuditEventFilter {
    pub event: Option<String>,
    pub principal: Option<String>,
    pub outcome: Option<String>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPackageRecord {
    pub id: String,
    pub package_token: String,
    pub application_id: String,
    pub bucket_name: String,
    pub object_key: String,
    pub manifest_id: String,
    pub expires_at: String,
    pub created_at: String,
    pub signature_algorithm: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventRecord {
    pub id: String,
    pub event: String,
    pub principal: Option<String>,
    pub outcome: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    pub enabled: bool,
    pub endpoint_path: String,
    pub bind_host: Option<String>,
    pub require_auth: bool,
    pub read_tools_enabled: bool,
    pub write_tools_enabled: bool,
    pub admin_tools_enabled: bool,
    pub expose_resources: bool,
    pub expose_prompts: bool,
    pub allow_localhost_only: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettingsUpdate {
    pub enabled: bool,
    pub endpoint_path: String,
    pub bind_host: Option<String>,
    pub require_auth: bool,
    pub read_tools_enabled: bool,
    pub write_tools_enabled: bool,
    pub admin_tools_enabled: bool,
    pub expose_resources: bool,
    pub expose_prompts: bool,
    pub allow_localhost_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAccessTokenSummary {
    pub id: String,
    pub name: String,
    pub token_prefix: String,
    pub active: bool,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedMcpAccessToken {
    pub token: McpAccessTokenSummary,
    pub secret: String,
}

#[derive(Debug, Clone)]
pub struct McpTokenAuthorization {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpActivityRecord {
    pub id: String,
    pub token_id: Option<String>,
    pub method: String,
    pub target: Option<String>,
    pub outcome: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub enabled: bool,
    pub endpoint: String,
    pub auth_required: bool,
    pub read_tools_enabled: bool,
    pub write_tools_enabled: bool,
    pub admin_tools_enabled: bool,
    pub resources_enabled: bool,
    pub prompts_enabled: bool,
    pub last_activity_at: Option<String>,
    pub active_sessions_count: i64,
    pub recent_calls_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrafficSummary {
    pub total_requests: i64,
    pub full_object_requests: i64,
    pub range_requests: i64,
    pub total_bytes_served: i64,
    pub fallback_events: i64,
    pub integrity_failures: i64,
    pub origin_offload_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserSummary {
    pub id: String,
    pub username: String,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AdminSessionRecord {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

impl Catalog {
    pub async fn initialize() -> anyhow::Result<Self> {
        let database_url = config::database_url_from_env()?;
        Self::initialize_with_url(&database_url).await
    }

    pub async fn initialize_with_url(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
            .context("failed to connect to PostgreSQL using PONTEMESH_DATABASE_URL")?;

        sqlx_core::migrate::Migrator::new(Path::new("./migrations"))
            .await
            .context("failed to load PostgreSQL migrations")?
            .run(&pool)
            .await
            .context("failed to run PostgreSQL migrations")?;

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn db_pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn database_connected(&self) -> bool {
        query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    pub async fn create_initial_admin_user(
        &self,
        username: &str,
        password_hash: &str,
    ) -> anyhow::Result<String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin setup transaction")?;
        let user_id: String = query(
            r#"
            INSERT INTO users (username, password_hash, role)
            VALUES ($1, $2, 'admin')
            RETURNING id::text
            "#,
        )
        .bind(username)
        .bind(password_hash)
        .fetch_one(&mut *tx)
        .await
        .context("failed to create initial admin user")?
        .get("id");

        record_audit_event_in_tx(
            &mut tx,
            "setup_completed",
            None,
            None,
            None,
            serde_json::json!({
                "principal": username,
                "outcome": "success",
                "detail": "initial admin user created"
            }),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit setup transaction")?;
        Ok(user_id)
    }

    pub async fn find_active_user_by_username(
        &self,
        username: &str,
    ) -> anyhow::Result<Option<UserRecord>> {
        let row = query(
            r#"
            SELECT id::text, username, password_hash, role
            FROM users
            WHERE username = $1 AND is_active = TRUE
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load user")?;

        Ok(row.map(|row| UserRecord {
            id: row.get("id"),
            username: row.get("username"),
            password_hash: row.get("password_hash"),
            role: row.get("role"),
        }))
    }

    pub async fn list_active_admin_users(&self) -> anyhow::Result<Vec<AdminUserSummary>> {
        let rows = query(
            "SELECT id::text, username, created_at, last_login_at FROM users WHERE role = 'admin' AND is_active = TRUE ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list active admin users")?;
        Ok(rows
            .into_iter()
            .map(|row| AdminUserSummary {
                id: row.get("id"),
                username: row.get("username"),
                created_at: format_datetime(row.get("created_at")),
                last_login_at: row
                    .try_get("last_login_at")
                    .ok()
                    .flatten()
                    .map(format_datetime),
            })
            .collect())
    }

    pub async fn create_admin_user(
        &self,
        username: &str,
        password_hash: &str,
    ) -> anyhow::Result<String> {
        let row = query("INSERT INTO users (username, password_hash, role) VALUES ($1, $2, 'admin') RETURNING id::text")
            .bind(username)
            .bind(password_hash)
            .fetch_one(&self.pool)
            .await
            .context("failed to create admin user")?;
        Ok(row.get("id"))
    }

    pub async fn update_admin_credentials(
        &self,
        user_id: &str,
        username: &str,
        password_hash: &str,
    ) -> anyhow::Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin credentials update transaction")?;
        let result = query("UPDATE users SET username = $2, password_hash = $3, updated_at = now() WHERE id = $1::uuid AND role = 'admin' AND is_active = TRUE")
            .bind(user_id)
            .bind(username)
            .bind(password_hash)
            .execute(&mut *tx)
            .await
            .context("failed to update admin credentials")?;
        if result.rows_affected() != 1 {
            bail!("active admin user not found");
        }
        query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1::uuid AND revoked_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("failed to revoke prior admin sessions")?;
        tx.commit()
            .await
            .context("failed to commit credentials update")
    }

    pub async fn first_active_admin_user(&self) -> anyhow::Result<Option<UserRecord>> {
        let row = query(
            r#"
            SELECT id::text, username, password_hash, role
            FROM users
            WHERE role = 'admin' AND is_active = TRUE
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("failed to load active admin user")?;

        Ok(row.map(|row| UserRecord {
            id: row.get("id"),
            username: row.get("username"),
            password_hash: row.get("password_hash"),
            role: row.get("role"),
        }))
    }

    pub async fn create_user_session(
        &self,
        user_id: &str,
        token_hash: &str,
        user_agent: Option<&str>,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin login transaction")?;
        query("UPDATE users SET last_login_at = now(), updated_at = now() WHERE id = $1::uuid")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("failed to update user login timestamp")?;

        query(
            r#"
            INSERT INTO sessions (
                user_id, session_token_hash, expires_at, user_agent, ip_address
            )
            VALUES ($1::uuid, $2, now() + interval '12 hours', $3, $4::inet)
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(user_agent)
        .bind(ip_address.map(|value| value.to_string()))
        .execute(&mut *tx)
        .await
        .context("failed to create session")?;

        tx.commit()
            .await
            .context("failed to commit login transaction")?;
        Ok(())
    }

    pub async fn find_admin_session_by_token_hash(
        &self,
        token_hash: &str,
    ) -> anyhow::Result<Option<AdminSessionRecord>> {
        let row = query(
            r#"
            SELECT u.id::text AS user_id, u.username, u.role
            FROM sessions s
            JOIN users u ON u.id = s.user_id
            WHERE s.session_token_hash = $1
              AND s.revoked_at IS NULL
              AND s.expires_at > now()
              AND u.is_active = TRUE
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load session")?;

        Ok(row.map(|row| AdminSessionRecord {
            user_id: row.get("user_id"),
            username: row.get("username"),
            role: row.get("role"),
        }))
    }

    pub async fn revoke_session_by_token_hash(&self, token_hash: &str) -> anyhow::Result<()> {
        query(
            "UPDATE sessions SET revoked_at = now() WHERE session_token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await
        .context("failed to revoke session")?;
        Ok(())
    }

    pub async fn list_buckets(&self) -> anyhow::Result<Vec<BucketSummary>> {
        let rows = query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(v.id)::bigint AS object_count,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_id = b.id AND o.deleted_at IS NULL
            LEFT JOIN object_versions v
                ON v.id = o.current_version_id AND NOT v.is_delete_marker
            WHERE b.deleted_at IS NULL
            GROUP BY b.id, b.name, b.created_at
            ORDER BY b.created_at DESC, b.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list buckets")?;

        Ok(rows
            .into_iter()
            .map(|row| BucketSummary {
                name: row.get("name"),
                created_at: format_datetime(row.get("created_at")),
                object_count: row.get("object_count"),
                total_bytes: row.get("total_bytes"),
            })
            .collect())
    }

    pub async fn list_buckets_page(
        &self,
        query_text: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> anyhow::Result<PaginatedBuckets> {
        let normalized_query = normalize_optional_name(query_text.unwrap_or(""));
        let total: i64 = query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM buckets b
            WHERE b.deleted_at IS NULL
              AND ($1::text IS NULL OR b.name ILIKE '%' || $1 || '%')
            "#,
        )
        .bind(normalized_query.as_deref())
        .fetch_one(&self.pool)
        .await
        .context("failed to count buckets")?;
        let total_pages = total_pages(total, page_size);
        let page = page.min(total_pages).max(1);
        let offset = i64::from(page.saturating_sub(1)) * i64::from(page_size);
        let limit = i64::from(page_size);
        let rows = query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(v.id)::bigint AS object_count,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_id = b.id AND o.deleted_at IS NULL
            LEFT JOIN object_versions v
                ON v.id = o.current_version_id AND NOT v.is_delete_marker
            WHERE b.deleted_at IS NULL
              AND ($1::text IS NULL OR b.name ILIKE '%' || $1 || '%')
            GROUP BY b.id, b.name, b.created_at
            ORDER BY b.created_at DESC, b.name ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(normalized_query.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("failed to list buckets")?;

        Ok(PaginatedBuckets {
            items: rows
                .into_iter()
                .map(|row| BucketSummary {
                    name: row.get("name"),
                    created_at: format_datetime(row.get("created_at")),
                    object_count: row.get("object_count"),
                    total_bytes: row.get("total_bytes"),
                })
                .collect(),
            page,
            page_size,
            total_items: total,
            total_pages,
        })
    }

    pub async fn get_bucket(&self, name: &str) -> anyhow::Result<Option<BucketSummary>> {
        validate_bucket_name(name)?;
        let row = query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(v.id)::bigint AS object_count,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_id = b.id AND o.deleted_at IS NULL
            LEFT JOIN object_versions v
                ON v.id = o.current_version_id AND NOT v.is_delete_marker
            WHERE b.name = $1 AND b.deleted_at IS NULL
            GROUP BY b.id, b.name, b.created_at
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load bucket")?;

        Ok(row.map(|row| BucketSummary {
            name: row.get("name"),
            created_at: format_datetime(row.get("created_at")),
            object_count: row.get("object_count"),
            total_bytes: row.get("total_bytes"),
        }))
    }

    pub async fn create_bucket(&self, name: &str) -> anyhow::Result<BucketSummary> {
        validate_bucket_name(name)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin bucket transaction")?;
        let row = query(
            r#"
            INSERT INTO buckets (name)
            VALUES ($1)
            RETURNING id::text, name, created_at
            "#,
        )
        .bind(name)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_unique_violation("bucket already exists"))?;

        query(
            r#"
            INSERT INTO bucket_policies (
                bucket_id, access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode
            )
            SELECT $1::uuid, access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode
            FROM bucket_policy_defaults
            WHERE singleton = TRUE
            ON CONFLICT (bucket_id) DO NOTHING
            "#,
        )
        .bind(row.get::<String, _>("id"))
        .execute(&mut *tx)
        .await
        .context("failed to create default bucket policy")?;

        tx.commit()
            .await
            .context("failed to commit bucket transaction")?;
        Ok(BucketSummary {
            name: row.get("name"),
            object_count: 0,
            total_bytes: 0,
            created_at: format_datetime(row.get("created_at")),
        })
    }

    pub async fn delete_bucket(&self, name: &str) -> anyhow::Result<()> {
        validate_bucket_name(name)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin bucket delete transaction")?;
        let bucket_id = bucket_id_in_tx(&mut tx, name).await?;
        let object_count: i64 = query_scalar(
            "SELECT COUNT(*)::bigint FROM objects WHERE bucket_id = $1::uuid AND deleted_at IS NULL",
        )
        .bind(&bucket_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to count bucket objects")?;
        if object_count > 0 {
            bail!("bucket must be empty before it can be deleted");
        }

        let result = query(
            "UPDATE buckets SET deleted_at = now(), updated_at = now() WHERE id = $1::uuid AND deleted_at IS NULL",
        )
        .bind(bucket_id)
        .execute(&mut *tx)
        .await
        .context("failed to delete bucket")?;
        if result.rows_affected() == 0 {
            bail!("bucket not found: {name}");
        }
        tx.commit()
            .await
            .context("failed to commit bucket delete")?;
        Ok(())
    }

    pub async fn get_bucket_policy(&self, bucket_name: &str) -> anyhow::Result<BucketPolicy> {
        validate_bucket_name(bucket_name)?;
        let row = query(
            r#"
            SELECT
                b.name,
                p.access_package_ttl_seconds,
                p.fragment_size_bytes,
                p.allow_replica_edge,
                p.allow_peer_sharing,
                p.source_selection_strategy,
                p.fragment_priority_strategy,
                p.failure_threshold,
                p.fallback_mode,
                p.s3_list_default_max_keys,
                p.s3_list_max_keys_limit,
                p.s3_list_allow_delimiter,
                p.s3_versioning_enabled,
                p.s3_object_tagging_enabled,
                p.s3_checksum_algorithm,
                p.s3_multipart_abort_days,
                p.s3_default_encryption_algorithm,
                p.s3_default_encryption_key_id,
                p.s3_object_lock_enabled,
                p.s3_object_lock_default_mode,
                p.s3_object_lock_default_retain_days,
                p.s3_lifecycle_rules,
                p.s3_resource_policy,
                p.s3_event_notifications,
                p.updated_at
            FROM bucket_policies p
            JOIN buckets b ON b.id = p.bucket_id
            WHERE b.name = $1 AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load bucket policy")?;

        match row {
            Some(row) => Ok(bucket_policy_from_row(row)),
            None => bail!("bucket not found: {bucket_name}"),
        }
    }

    pub async fn get_bucket_policy_defaults(&self) -> anyhow::Result<BucketPolicyDefaults> {
        let row = query(
            r#"
            SELECT access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode, updated_at
            FROM bucket_policy_defaults
            WHERE singleton = TRUE
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to load bucket policy defaults")?;
        Ok(bucket_policy_defaults_from_row(row))
    }

    pub async fn update_bucket_policy_defaults(
        &self,
        update: BucketPolicyDefaultsUpdate,
    ) -> anyhow::Result<BucketPolicyDefaults> {
        validate_bucket_policy_defaults(&update)?;
        let row = query(
            r#"
            UPDATE bucket_policy_defaults SET
                access_package_ttl_seconds = $1,
                fragment_size_bytes = $2,
                allow_replica_edge = $3,
                allow_peer_sharing = $4,
                source_selection_strategy = $5,
                fragment_priority_strategy = $6,
                failure_threshold = $7,
                fallback_mode = $8,
                updated_at = now()
            WHERE singleton = TRUE
            RETURNING access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode, updated_at
            "#,
        )
        .bind(update.access_package_ttl_seconds)
        .bind(update.fragment_size_bytes)
        .bind(update.allow_replica_edge)
        .bind(update.allow_peer_sharing)
        .bind(&update.source_selection_strategy)
        .bind(&update.fragment_priority_strategy)
        .bind(update.failure_threshold)
        .bind(&update.fallback_mode)
        .fetch_one(&self.pool)
        .await
        .context("failed to update bucket policy defaults")?;
        Ok(bucket_policy_defaults_from_row(row))
    }

    pub async fn bulk_update_bucket_policy(
        &self,
        all_buckets: bool,
        bucket_names: &[String],
        update: BucketPolicyDefaultsUpdate,
    ) -> anyhow::Result<Vec<String>> {
        validate_bucket_policy_defaults(&update)?;
        let mut names = bucket_names.to_vec();
        names.sort();
        names.dedup();
        if !all_buckets && names.is_empty() {
            bail!("select at least one bucket");
        }
        for name in &names {
            validate_bucket_name(name)?;
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin bulk policy transaction")?;
        let rows = query(
            r#"
            UPDATE bucket_policies p SET
                access_package_ttl_seconds = $1,
                fragment_size_bytes = $2,
                allow_replica_edge = $3,
                allow_peer_sharing = $4,
                source_selection_strategy = $5,
                fragment_priority_strategy = $6,
                failure_threshold = $7,
                fallback_mode = $8,
                updated_at = now()
            FROM buckets b
            WHERE p.bucket_id = b.id
              AND b.deleted_at IS NULL
              AND ($9 OR b.name = ANY($10))
            RETURNING b.name
            "#,
        )
        .bind(update.access_package_ttl_seconds)
        .bind(update.fragment_size_bytes)
        .bind(update.allow_replica_edge)
        .bind(update.allow_peer_sharing)
        .bind(&update.source_selection_strategy)
        .bind(&update.fragment_priority_strategy)
        .bind(update.failure_threshold)
        .bind(&update.fallback_mode)
        .bind(all_buckets)
        .bind(&names)
        .fetch_all(&mut *tx)
        .await
        .context("failed to update bucket policies in bulk")?;
        let mut updated_names = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect::<Vec<_>>();
        updated_names.sort();
        if !all_buckets && updated_names.len() != names.len() {
            bail!("one or more selected buckets do not exist");
        }
        tx.commit()
            .await
            .context("failed to commit bulk policy transaction")?;
        Ok(updated_names)
    }

    pub async fn list_bucket_policies(&self) -> anyhow::Result<Vec<BucketPolicy>> {
        let rows = query(
            r#"
            SELECT
                b.name,
                p.access_package_ttl_seconds,
                p.fragment_size_bytes,
                p.allow_replica_edge,
                p.allow_peer_sharing,
                p.source_selection_strategy,
                p.fragment_priority_strategy,
                p.failure_threshold,
                p.fallback_mode,
                p.s3_list_default_max_keys,
                p.s3_list_max_keys_limit,
                p.s3_list_allow_delimiter,
                p.s3_versioning_enabled,
                p.s3_object_tagging_enabled,
                p.s3_checksum_algorithm,
                p.s3_multipart_abort_days,
                p.s3_default_encryption_algorithm,
                p.s3_default_encryption_key_id,
                p.s3_object_lock_enabled,
                p.s3_object_lock_default_mode,
                p.s3_object_lock_default_retain_days,
                p.s3_lifecycle_rules,
                p.s3_resource_policy,
                p.s3_event_notifications,
                p.updated_at
            FROM bucket_policies p
            JOIN buckets b ON b.id = p.bucket_id
            WHERE b.deleted_at IS NULL
            ORDER BY b.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list bucket policies")?;
        Ok(rows.into_iter().map(bucket_policy_from_row).collect())
    }

    pub async fn update_bucket_policy(
        &self,
        bucket_name: &str,
        update: BucketPolicyUpdate,
    ) -> anyhow::Result<BucketPolicy> {
        validate_bucket_name(bucket_name)?;
        validate_bucket_policy(&update)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin bucket policy transaction")?;
        let bucket_id = bucket_id_in_tx(&mut tx, bucket_name).await?;
        let row = query(
            r#"
            INSERT INTO bucket_policies (
                bucket_id, access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode,
                s3_list_default_max_keys, s3_list_max_keys_limit,
                s3_list_allow_delimiter, s3_versioning_enabled,
                s3_object_tagging_enabled, s3_checksum_algorithm,
                s3_multipart_abort_days, s3_default_encryption_algorithm,
                s3_default_encryption_key_id, s3_object_lock_enabled,
                s3_object_lock_default_mode, s3_object_lock_default_retain_days,
                s3_lifecycle_rules, s3_resource_policy, s3_event_notifications, updated_at
            )
            VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                $17, $18, $19, $20, $21, $22, $23, $24, now())
            ON CONFLICT (bucket_id) DO UPDATE SET
                access_package_ttl_seconds = EXCLUDED.access_package_ttl_seconds,
                fragment_size_bytes = EXCLUDED.fragment_size_bytes,
                allow_replica_edge = EXCLUDED.allow_replica_edge,
                allow_peer_sharing = EXCLUDED.allow_peer_sharing,
                source_selection_strategy = EXCLUDED.source_selection_strategy,
                fragment_priority_strategy = EXCLUDED.fragment_priority_strategy,
                failure_threshold = EXCLUDED.failure_threshold,
                fallback_mode = EXCLUDED.fallback_mode,
                s3_list_default_max_keys = EXCLUDED.s3_list_default_max_keys,
                s3_list_max_keys_limit = EXCLUDED.s3_list_max_keys_limit,
                s3_list_allow_delimiter = EXCLUDED.s3_list_allow_delimiter,
                s3_versioning_enabled = EXCLUDED.s3_versioning_enabled,
                s3_object_tagging_enabled = EXCLUDED.s3_object_tagging_enabled,
                s3_checksum_algorithm = EXCLUDED.s3_checksum_algorithm,
                s3_multipart_abort_days = EXCLUDED.s3_multipart_abort_days,
                s3_default_encryption_algorithm = EXCLUDED.s3_default_encryption_algorithm,
                s3_default_encryption_key_id = EXCLUDED.s3_default_encryption_key_id,
                s3_object_lock_enabled = EXCLUDED.s3_object_lock_enabled,
                s3_object_lock_default_mode = EXCLUDED.s3_object_lock_default_mode,
                s3_object_lock_default_retain_days = EXCLUDED.s3_object_lock_default_retain_days,
                s3_lifecycle_rules = EXCLUDED.s3_lifecycle_rules,
                s3_resource_policy = EXCLUDED.s3_resource_policy,
                s3_event_notifications = EXCLUDED.s3_event_notifications,
                updated_at = now()
            RETURNING access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing,
                source_selection_strategy, fragment_priority_strategy,
                failure_threshold, fallback_mode,
                s3_list_default_max_keys, s3_list_max_keys_limit,
                s3_list_allow_delimiter, s3_versioning_enabled,
                s3_object_tagging_enabled, s3_checksum_algorithm,
                s3_multipart_abort_days, s3_default_encryption_algorithm,
                s3_default_encryption_key_id, s3_object_lock_enabled,
                s3_object_lock_default_mode, s3_object_lock_default_retain_days,
                s3_lifecycle_rules, s3_resource_policy, s3_event_notifications, updated_at
            "#,
        )
        .bind(bucket_id)
        .bind(update.access_package_ttl_seconds)
        .bind(update.fragment_size_bytes)
        .bind(update.allow_replica_edge)
        .bind(update.allow_peer_sharing)
        .bind(&update.source_selection_strategy)
        .bind(&update.fragment_priority_strategy)
        .bind(update.failure_threshold)
        .bind(&update.fallback_mode)
        .bind(update.s3_list_default_max_keys)
        .bind(update.s3_list_max_keys_limit)
        .bind(update.s3_list_allow_delimiter)
        .bind(update.s3_versioning_enabled)
        .bind(update.s3_object_tagging_enabled)
        .bind(&update.s3_checksum_algorithm)
        .bind(update.s3_multipart_abort_days)
        .bind(&update.s3_default_encryption_algorithm)
        .bind(update.s3_default_encryption_key_id.as_deref())
        .bind(update.s3_object_lock_enabled)
        .bind(update.s3_object_lock_default_mode.as_deref())
        .bind(update.s3_object_lock_default_retain_days)
        .bind(&update.s3_lifecycle_rules)
        .bind(&update.s3_resource_policy)
        .bind(&update.s3_event_notifications)
        .fetch_one(&mut *tx)
        .await
        .context("failed to update bucket policy")?;

        tx.commit()
            .await
            .context("failed to commit bucket policy")?;
        Ok(BucketPolicy {
            bucket_name: bucket_name.to_owned(),
            access_package_ttl_seconds: row.get("access_package_ttl_seconds"),
            fragment_size_bytes: row.get("fragment_size_bytes"),
            allow_replica_edge: row.get("allow_replica_edge"),
            allow_peer_sharing: row.get("allow_peer_sharing"),
            source_selection_strategy: row.get("source_selection_strategy"),
            fragment_priority_strategy: row.get("fragment_priority_strategy"),
            failure_threshold: row.get("failure_threshold"),
            fallback_mode: row.get("fallback_mode"),
            s3_list_default_max_keys: row.get("s3_list_default_max_keys"),
            s3_list_max_keys_limit: row.get("s3_list_max_keys_limit"),
            s3_list_allow_delimiter: row.get("s3_list_allow_delimiter"),
            s3_versioning_enabled: row.get("s3_versioning_enabled"),
            s3_object_tagging_enabled: row.get("s3_object_tagging_enabled"),
            s3_checksum_algorithm: row.get("s3_checksum_algorithm"),
            s3_multipart_abort_days: row.get("s3_multipart_abort_days"),
            s3_default_encryption_algorithm: row.get("s3_default_encryption_algorithm"),
            s3_default_encryption_key_id: row.get("s3_default_encryption_key_id"),
            s3_object_lock_enabled: row.get("s3_object_lock_enabled"),
            s3_object_lock_default_mode: row.get("s3_object_lock_default_mode"),
            s3_object_lock_default_retain_days: row.get("s3_object_lock_default_retain_days"),
            s3_lifecycle_rules: row.get("s3_lifecycle_rules"),
            s3_resource_policy: row.get("s3_resource_policy"),
            s3_event_notifications: row.get("s3_event_notifications"),
            updated_at: format_datetime(row.get("updated_at")),
        })
    }

    pub async fn list_objects_v2(
        &self,
        bucket_name: &str,
        options: ListObjectsV2Options,
    ) -> anyhow::Result<S3ListObjectsPage> {
        validate_bucket_name(bucket_name)?;
        let marker = options.continuation_token.or(options.start_after.clone());
        let prefix = options.prefix.unwrap_or_default();
        let rows = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash,
                v.s3_version_id, v.is_delete_marker,
                v.created_at, v.created_at AS updated_at, o.state,
                v.created_by
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE b.name = $1
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND NOT v.is_delete_marker
              AND o.object_key LIKE $2 || '%'
              AND ($3::text IS NULL OR o.object_key > $3)
            ORDER BY o.object_key ASC
            "#,
        )
        .bind(bucket_name)
        .bind(&prefix)
        .bind(marker.as_deref())
        .fetch_all(&self.pool)
        .await
        .context("failed to list objects")?;

        let max_keys = options.max_keys.max(1);
        let delimiter = options.delimiter.filter(|value| !value.is_empty());
        let mut items = Vec::new();
        let mut common_prefixes = Vec::<String>::new();
        let mut emitted = 0_i64;
        let mut last_emitted_key: Option<String> = marker;
        let mut is_truncated = false;

        for row in rows {
            let object = object_summary_from_row(row);
            let candidate_prefix = delimiter
                .as_deref()
                .and_then(|delimiter| common_prefix_for_key(&prefix, delimiter, &object.key));
            let will_emit = match candidate_prefix {
                Some(common_prefix) => {
                    if common_prefixes.iter().any(|value| value == &common_prefix) {
                        false
                    } else {
                        if emitted >= max_keys {
                            is_truncated = true;
                            break;
                        }
                        common_prefixes.push(common_prefix);
                        true
                    }
                }
                None => {
                    if emitted >= max_keys {
                        is_truncated = true;
                        break;
                    }
                    items.push(object.clone());
                    true
                }
            };
            if will_emit {
                emitted += 1;
                last_emitted_key = Some(object.key);
            }
        }

        Ok(S3ListObjectsPage {
            items,
            common_prefixes,
            key_count: usize::try_from(emitted).unwrap_or(usize::MAX),
            max_keys,
            is_truncated,
            next_continuation_token: is_truncated.then_some(last_emitted_key).flatten(),
            start_after: options.start_after,
        })
    }

    pub async fn list_objects_page(
        &self,
        bucket_name: &str,
        query_text: Option<&str>,
        prefix: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> anyhow::Result<PaginatedObjects> {
        validate_bucket_name(bucket_name)?;
        let normalized_query = normalize_optional_name(query_text.unwrap_or(""));
        let effective_prefix: Option<String> = prefix.filter(|p| !p.is_empty()).map(|p| {
            if p.ends_with('/') {
                p.to_owned()
            } else {
                format!("{p}/")
            }
        });

        let bucket_id: Option<String> =
            query_scalar("SELECT id::text FROM buckets WHERE name = $1 AND deleted_at IS NULL")
                .bind(bucket_name)
                .fetch_optional(&self.pool)
                .await
                .context("failed to load bucket")?;
        let bucket_id =
            bucket_id.ok_or_else(|| anyhow::anyhow!("bucket not found: {bucket_name}"))?;

        let rows = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash,
                v.s3_version_id, v.is_delete_marker,
                v.created_at, v.created_at AS updated_at, o.state,
                v.created_by
            FROM objects o
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE o.bucket_id = $1::uuid
              AND o.deleted_at IS NULL
              AND NOT v.is_delete_marker
              AND ($2::text IS NULL OR o.object_key LIKE $2 || '%')
              AND ($3::text IS NULL OR o.object_key ILIKE '%' || $3 || '%')
            ORDER BY o.object_key ASC
            "#,
        )
        .bind(&bucket_id)
        .bind(effective_prefix.as_deref())
        .bind(normalized_query.as_deref())
        .fetch_all(&self.pool)
        .await
        .context("failed to list objects")?;

        let delimiter = "/";
        let prefix_str = effective_prefix.as_deref().unwrap_or("");
        let (paged_items, common_prefixes, total, total_pages, page) = split_and_paginate(
            rows.into_iter().map(object_summary_from_row).collect(),
            prefix_str,
            normalized_query.is_none().then_some(delimiter),
            page,
            page_size,
        );

        Ok(PaginatedObjects {
            items: paged_items,
            common_prefixes,
            page,
            page_size,
            total_items: total,
            total_pages,
        })
    }

    pub async fn insert_object(&self, object: NewObject) -> anyhow::Result<ObjectSummary> {
        self.insert_or_replace_object(object, false, None).await
    }

    pub async fn put_object_with_audit(
        &self,
        object: NewObject,
        principal: &str,
        detail: &str,
    ) -> anyhow::Result<ObjectSummary> {
        self.insert_or_replace_object(
            object,
            true,
            Some(ObjectAuditEvent {
                event: "s3_object_put",
                principal,
                outcome: "success",
                detail,
            }),
        )
        .await
    }

    async fn insert_or_replace_object(
        &self,
        object: NewObject,
        replace_existing: bool,
        audit_event: Option<ObjectAuditEvent<'_>>,
    ) -> anyhow::Result<ObjectSummary> {
        validate_bucket_name(&object.bucket_name)?;
        validate_object_key(&object.key)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin object transaction")?;
        let bucket_id = bucket_id_in_tx(&mut tx, &object.bucket_name).await?;

        let object_id = if replace_existing {
            self.ensure_object_can_be_deleted(&object.bucket_name, &object.key, None)
                .await?;
            upsert_object_in_tx(&mut tx, &bucket_id, &object.key).await?
        } else {
            insert_object_shell_in_tx(&mut tx, &bucket_id, &object.key).await?
        };

        let version_number: i64 = query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM object_versions WHERE object_id = $1::uuid",
        )
        .bind(&object_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to allocate object version")?;

        let s3_version_id = uuid::Uuid::new_v4().to_string();
        let version_row = query(
            r#"
            INSERT INTO object_versions (
                object_id, version_number, size_bytes, content_type,
                hash_algorithm, object_hash, storage_path, s3_version_id,
                checksum_sha256, checksum_crc32, encryption_algorithm,
                encryption_key_id, encryption_nonce, object_lock_mode,
                retain_until, legal_hold, user_metadata, created_by
            )
            VALUES ($1::uuid, $2, $3, $4, 'SHA-256', $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING id::text, created_at
            "#,
        )
        .bind(&object_id)
        .bind(version_number)
        .bind(object.size_bytes)
        .bind(&object.content_type)
        .bind(&object.sha256)
        .bind(&object.storage_path)
        .bind(&s3_version_id)
        .bind(object.checksum_sha256.as_deref())
        .bind(object.checksum_crc32.as_deref())
        .bind(object.encryption_algorithm.as_deref())
        .bind(object.encryption_key_id.as_deref())
        .bind(object.encryption_nonce.as_deref())
        .bind(object.object_lock_mode.as_deref())
        .bind(object.retain_until)
        .bind(object.legal_hold)
        .bind(object.user_metadata.as_ref())
        .bind(object.created_by.as_deref())
        .fetch_one(&mut *tx)
        .await
        .context("failed to register object version")?;
        let version_id = version_row.get::<String, _>("id");

        let manifest_id = query(
            r#"
            INSERT INTO object_manifests (
                object_version_id, fragment_size_bytes, object_hash_algorithm, object_hash
            )
            VALUES ($1::uuid, $2, 'SHA-256', $3)
            RETURNING id::text
            "#,
        )
        .bind(&version_id)
        .bind(object.manifest.fragment_size_bytes)
        .bind(&object.sha256)
        .fetch_one(&mut *tx)
        .await
        .context("failed to register object manifest")?
        .get::<String, _>("id");

        for fragment in &object.manifest.fragments {
            query(
                r#"
                INSERT INTO object_manifest_fragments (
                    manifest_id, fragment_index, byte_range_start, byte_range_end,
                    size_bytes, hash_algorithm, fragment_hash, priority
                )
                VALUES ($1::uuid, $2, $3, $4, $5, 'SHA-256', $6, $7)
                "#,
            )
            .bind(&manifest_id)
            .bind(fragment.index)
            .bind(fragment.byte_range_start)
            .bind(fragment.byte_range_end)
            .bind(fragment.size_bytes)
            .bind(&fragment.sha256)
            .bind(&fragment.priority)
            .execute(&mut *tx)
            .await
            .context("failed to register object manifest fragment")?;
        }

        query(
            r#"
            UPDATE objects
            SET current_version_id = $1::uuid, state = 'AVAILABLE',
                deleted_at = NULL, updated_at = now()
            WHERE id = $2::uuid
            "#,
        )
        .bind(&version_id)
        .bind(&object_id)
        .execute(&mut *tx)
        .await
        .context("failed to update current object version")?;

        if let Some(audit_event) = audit_event {
            record_audit_event_in_tx(
                &mut tx,
                audit_event.event,
                None,
                None,
                None,
                serde_json::json!({
                    "principal": audit_event.principal,
                    "outcome": audit_event.outcome,
                    "detail": audit_event.detail
                }),
            )
            .await?;
        }

        tx.commit()
            .await
            .context("failed to commit object transaction")?;
        Ok(ObjectSummary {
            key: object.key,
            size_bytes: object.size_bytes,
            content_type: object.content_type,
            sha256: object.sha256,
            version_id: Some(s3_version_id),
            is_delete_marker: false,
            created_at: format_datetime(version_row.get("created_at")),
            updated_at: format_datetime(version_row.get("created_at")),
            state: "AVAILABLE".to_owned(),
            created_by: object.created_by,
        })
    }

    pub async fn get_object_record(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<ObjectRecord>> {
        self.get_object_record_version(bucket_name, object_key, None)
            .await
    }

    pub async fn get_object_record_version(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
    ) -> anyhow::Result<Option<ObjectRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash,
                v.storage_path, v.s3_version_id, v.is_delete_marker,
                v.checksum_sha256, v.checksum_crc32, v.encryption_algorithm,
                v.encryption_key_id, v.encryption_nonce, v.object_lock_mode,
                v.retain_until, v.legal_hold, v.created_at, o.state,
                v.user_metadata, v.created_by
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.object_id = o.id
            WHERE b.name = $1 AND o.object_key = $2
              AND b.deleted_at IS NULL AND o.deleted_at IS NULL
              AND (($3::text IS NULL AND v.id = o.current_version_id) OR v.s3_version_id = $3)
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load object")?;

        Ok(row.map(object_record_from_row))
    }

    pub async fn list_object_tags(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Vec<S3ObjectTag>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let rows = query(
            r#"
            SELECT t.tag_key, t.tag_value
            FROM s3_object_tags t
            JOIN objects o ON o.id = t.object_id
            JOIN buckets b ON b.id = o.bucket_id
            WHERE b.name = $1
              AND b.deleted_at IS NULL
              AND o.object_key = $2
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
            ORDER BY t.tag_key ASC
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list object tags")?;
        Ok(rows
            .into_iter()
            .map(|row| S3ObjectTag {
                key: row.get("tag_key"),
                value: row.get("tag_value"),
            })
            .collect())
    }

    pub async fn replace_object_tags(
        &self,
        bucket_name: &str,
        object_key: &str,
        tags: Vec<S3ObjectTag>,
    ) -> anyhow::Result<Vec<S3ObjectTag>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        validate_s3_object_tags(&tags)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin object tag transaction")?;
        let object_id: Option<String> = query_scalar(
            r#"
            SELECT o.id::text
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            WHERE b.name = $1
              AND b.deleted_at IS NULL
              AND o.object_key = $2
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to load object for tagging")?;
        let object_id = object_id.ok_or_else(|| anyhow::anyhow!("object not found"))?;

        query("DELETE FROM s3_object_tags WHERE object_id = $1::uuid")
            .bind(&object_id)
            .execute(&mut *tx)
            .await
            .context("failed to clear object tags")?;
        for tag in &tags {
            query(
                r#"
                INSERT INTO s3_object_tags (object_id, tag_key, tag_value)
                VALUES ($1::uuid, $2, $3)
                "#,
            )
            .bind(&object_id)
            .bind(&tag.key)
            .bind(&tag.value)
            .execute(&mut *tx)
            .await
            .context("failed to store object tag")?;
        }
        tx.commit().await.context("failed to commit object tags")?;
        Ok(tags)
    }

    pub async fn delete_object_tags(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<()> {
        self.replace_object_tags(bucket_name, object_key, Vec::new())
            .await?;
        Ok(())
    }

    pub async fn get_object_manifest(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<ObjectManifest>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = query(
            r#"
            SELECT
                m.id::text AS manifest_id,
                o.id::text AS object_id,
                b.name AS bucket,
                o.object_key,
                v.object_hash,
                v.size_bytes,
                v.content_type,
                m.object_hash_algorithm,
                m.fragment_size_bytes,
                o.state,
                m.created_at,
                m.signature_algorithm,
                m.signature
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            WHERE b.name = $1
              AND o.object_key = $2
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load object manifest")?;

        let Some(row) = row else {
            return Ok(None);
        };

        let manifest_id = row.get::<String, _>("manifest_id");
        let fragments = query(
            r#"
            SELECT
                fragment_index, byte_range_start, byte_range_end, size_bytes,
                hash_algorithm, fragment_hash, priority
            FROM object_manifest_fragments
            WHERE manifest_id = $1::uuid
            ORDER BY fragment_index ASC
            "#,
        )
        .bind(&manifest_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load object manifest fragments")?;

        Ok(Some(ObjectManifest {
            manifest_id: manifest_id.clone(),
            object_id: row.get("object_id"),
            bucket: row.get("bucket"),
            key: row.get("object_key"),
            version: row.get("object_hash"),
            total_size_bytes: row.get("size_bytes"),
            content_type: row.get("content_type"),
            object_hash_algorithm: row.get("object_hash_algorithm"),
            object_sha256: row.get("object_hash"),
            fragment_size_bytes: row.get("fragment_size_bytes"),
            availability_state: row.get("state"),
            created_at: format_datetime(row.get("created_at")),
            signature_algorithm: row.get("signature_algorithm"),
            signature: row.get("signature"),
            fragments: fragments
                .into_iter()
                .map(|fragment| {
                    let index = fragment.get("fragment_index");
                    let sha256 = fragment.get("fragment_hash");
                    ObjectManifestFragment {
                        index,
                        fragment_id: format!("{manifest_id}:{index}:{sha256}"),
                        byte_range_start: fragment.get("byte_range_start"),
                        byte_range_end: fragment.get("byte_range_end"),
                        size_bytes: fragment.get("size_bytes"),
                        hash_algorithm: fragment.get("hash_algorithm"),
                        sha256,
                        priority: fragment.get("priority"),
                    }
                })
                .collect(),
        }))
    }

    pub async fn create_multipart_upload(
        &self,
        bucket_name: &str,
        object_key: &str,
        content_type: &str,
        initiated_by: &str,
        user_metadata: Option<serde_json::Value>,
    ) -> anyhow::Result<MultipartUploadRecord> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        if content_type.trim().is_empty() {
            bail!("content type cannot be empty");
        }
        let row = query(
            r#"
            INSERT INTO s3_multipart_uploads (bucket_id, object_key, content_type, initiated_by, user_metadata)
            SELECT id, $2, $3, $4, $5
            FROM buckets
            WHERE name = $1 AND deleted_at IS NULL
            RETURNING id::text AS upload_id, $1::text AS bucket_name, object_key,
                content_type, initiated_at, user_metadata
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .bind(content_type)
        .bind(initiated_by)
        .bind(user_metadata.as_ref())
        .fetch_optional(&self.pool)
        .await
        .context("failed to create multipart upload")?;
        let Some(row) = row else {
            bail!("bucket not found: {bucket_name}");
        };
        Ok(multipart_upload_from_row(row))
    }

    pub async fn get_active_multipart_upload(
        &self,
        bucket_name: &str,
        object_key: &str,
        upload_id: &str,
    ) -> anyhow::Result<Option<MultipartUploadRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = query(
            r#"
            SELECT u.id::text AS upload_id, b.name AS bucket_name, u.object_key,
                u.content_type, u.initiated_at, u.user_metadata
            FROM s3_multipart_uploads u
            JOIN buckets b ON b.id = u.bucket_id
            WHERE u.id = $1::uuid
              AND b.name = $2
              AND u.object_key = $3
              AND b.deleted_at IS NULL
              AND u.completed_at IS NULL
              AND u.aborted_at IS NULL
            "#,
        )
        .bind(upload_id)
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load multipart upload")?;
        Ok(row.map(multipart_upload_from_row))
    }

    pub async fn list_active_multipart_uploads(
        &self,
        bucket_name: &str,
    ) -> anyhow::Result<Vec<MultipartUploadRecord>> {
        validate_bucket_name(bucket_name)?;
        let rows = query(
            r#"
            SELECT u.id::text AS upload_id, b.name AS bucket_name, u.object_key,
                u.content_type, u.initiated_at, u.user_metadata
            FROM s3_multipart_uploads u
            JOIN buckets b ON b.id = u.bucket_id
            WHERE b.name = $1
              AND b.deleted_at IS NULL
              AND u.completed_at IS NULL
              AND u.aborted_at IS NULL
            ORDER BY u.initiated_at DESC, u.object_key ASC
            "#,
        )
        .bind(bucket_name)
        .fetch_all(&self.pool)
        .await
        .context("failed to list multipart uploads")?;
        Ok(rows.into_iter().map(multipart_upload_from_row).collect())
    }

    pub async fn record_multipart_part(
        &self,
        bucket_name: &str,
        object_key: &str,
        upload_id: &str,
        part: MultipartPartRecord,
    ) -> anyhow::Result<MultipartPartRecord> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        if !(1..=10_000).contains(&part.part_number) {
            bail!("partNumber must be between 1 and 10000");
        }
        if part.size_bytes < 0 {
            bail!("multipart part size cannot be negative");
        }
        let row = query(
            r#"
            WITH target AS (
                SELECT u.id
                FROM s3_multipart_uploads u
                JOIN buckets b ON b.id = u.bucket_id
                WHERE u.id = $1::uuid
                  AND b.name = $2
                  AND u.object_key = $3
                  AND b.deleted_at IS NULL
                  AND u.completed_at IS NULL
                  AND u.aborted_at IS NULL
            )
            INSERT INTO s3_multipart_upload_parts (
                upload_id, part_number, etag, size_bytes, storage_path
            )
            SELECT id, $4, $5, $6, $7
            FROM target
            ON CONFLICT (upload_id, part_number) DO UPDATE
            SET etag = EXCLUDED.etag,
                size_bytes = EXCLUDED.size_bytes,
                storage_path = EXCLUDED.storage_path,
                uploaded_at = now()
            RETURNING part_number, etag, size_bytes, storage_path, uploaded_at
            "#,
        )
        .bind(upload_id)
        .bind(bucket_name)
        .bind(object_key)
        .bind(part.part_number)
        .bind(&part.etag)
        .bind(part.size_bytes)
        .bind(&part.storage_path)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record multipart part")?;
        let Some(row) = row else {
            bail!("multipart upload not found or no longer active");
        };
        Ok(multipart_part_from_row(row))
    }

    pub async fn list_multipart_parts(
        &self,
        bucket_name: &str,
        object_key: &str,
        upload_id: &str,
    ) -> anyhow::Result<Vec<MultipartPartRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let rows = query(
            r#"
            SELECT p.part_number, p.etag, p.size_bytes, p.storage_path, p.uploaded_at
            FROM s3_multipart_upload_parts p
            JOIN s3_multipart_uploads u ON u.id = p.upload_id
            JOIN buckets b ON b.id = u.bucket_id
            WHERE u.id = $1::uuid
              AND b.name = $2
              AND u.object_key = $3
              AND b.deleted_at IS NULL
              AND u.completed_at IS NULL
              AND u.aborted_at IS NULL
            ORDER BY p.part_number ASC
            "#,
        )
        .bind(upload_id)
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list multipart parts")?;
        Ok(rows.into_iter().map(multipart_part_from_row).collect())
    }

    pub async fn complete_multipart_upload(&self, upload_id: &str) -> anyhow::Result<()> {
        let result = query(
            r#"
            UPDATE s3_multipart_uploads
            SET completed_at = now()
            WHERE id = $1::uuid
              AND completed_at IS NULL
              AND aborted_at IS NULL
            "#,
        )
        .bind(upload_id)
        .execute(&self.pool)
        .await
        .context("failed to complete multipart upload")?;
        if result.rows_affected() == 0 {
            bail!("multipart upload not found or no longer active");
        }
        Ok(())
    }

    pub async fn abort_multipart_upload(&self, upload_id: &str) -> anyhow::Result<()> {
        let result = query(
            r#"
            UPDATE s3_multipart_uploads
            SET aborted_at = now()
            WHERE id = $1::uuid
              AND completed_at IS NULL
              AND aborted_at IS NULL
            "#,
        )
        .bind(upload_id)
        .execute(&self.pool)
        .await
        .context("failed to abort multipart upload")?;
        if result.rows_affected() == 0 {
            bail!("multipart upload not found or no longer active");
        }
        Ok(())
    }

    pub async fn delete_object(&self, bucket_name: &str, object_key: &str) -> anyhow::Result<()> {
        let policy = self.get_bucket_policy(bucket_name).await?;
        if policy.s3_versioning_enabled {
            self.insert_delete_marker(bucket_name, object_key).await?;
            return Ok(());
        }
        self.ensure_object_can_be_deleted(bucket_name, object_key, None)
            .await?;
        self.set_object_state(bucket_name, object_key, "DELETED", true)
            .await
    }

    pub async fn revoke_object(&self, bucket_name: &str, object_key: &str) -> anyhow::Result<()> {
        self.set_object_state(bucket_name, object_key, "REVOKED", false)
            .await
    }

    async fn set_object_state(
        &self,
        bucket_name: &str,
        object_key: &str,
        state: &str,
        deleted: bool,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin object state transaction")?;
        let row = query(
            r#"
            SELECT o.id::text
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            WHERE b.name = $1 AND o.object_key = $2
              AND b.deleted_at IS NULL AND o.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to load object for state change")?;
        let Some(row) = row else {
            bail!("object not found: {object_key}");
        };
        let deleted_at = if deleted { "now()" } else { "NULL" };
        let update_sql = format!(
            "UPDATE objects SET state = $1, deleted_at = {deleted_at}, updated_at = now() WHERE id = $2::uuid"
        );
        query(&update_sql)
            .bind(state)
            .bind(row.get::<String, _>("id"))
            .execute(&mut *tx)
            .await
            .context("failed to update object state")?;
        tx.commit().await.context("failed to commit object state")?;
        Ok(())
    }

    pub async fn ensure_object_can_be_deleted(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
    ) -> anyhow::Result<()> {
        if let Some(object) = self
            .get_object_record_version(bucket_name, object_key, version_id)
            .await?
        {
            if object.legal_hold {
                bail!("object version is protected by legal hold");
            }
            if let Some(retain_until) = object.retain_until.as_deref() {
                let retain_until = chrono::DateTime::parse_from_rfc3339(retain_until)
                    .context("stored retention timestamp is invalid")?
                    .with_timezone(&chrono::Utc);
                if retain_until > chrono::Utc::now() {
                    bail!("object version is protected by retention");
                }
            }
        }
        Ok(())
    }

    pub async fn insert_delete_marker(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<String> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        self.ensure_object_can_be_deleted(bucket_name, object_key, None)
            .await?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin delete marker transaction")?;
        let bucket_id = bucket_id_in_tx(&mut tx, bucket_name).await?;
        let object_id = upsert_object_in_tx(&mut tx, &bucket_id, object_key).await?;
        let version_number: i64 = query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM object_versions WHERE object_id = $1::uuid",
        )
        .bind(&object_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to allocate delete marker version")?;
        let s3_version_id = uuid::Uuid::new_v4().to_string();
        let row = query(
            r#"
            INSERT INTO object_versions (
                object_id, version_number, size_bytes, content_type,
                hash_algorithm, object_hash, storage_path, s3_version_id, is_delete_marker
            )
            VALUES ($1::uuid, $2, 0, 'application/octet-stream', 'SHA-256', '', '', $3, TRUE)
            RETURNING id::text
            "#,
        )
        .bind(&object_id)
        .bind(version_number)
        .bind(&s3_version_id)
        .fetch_one(&mut *tx)
        .await
        .context("failed to insert delete marker")?;
        query(
            r#"
            UPDATE objects
            SET current_version_id = $1::uuid, state = 'DELETED', deleted_at = NULL, updated_at = now()
            WHERE id = $2::uuid
            "#,
        )
        .bind(row.get::<String, _>("id"))
        .bind(&object_id)
        .execute(&mut *tx)
        .await
        .context("failed to publish delete marker")?;
        tx.commit()
            .await
            .context("failed to commit delete marker")?;
        Ok(s3_version_id)
    }

    pub async fn list_object_versions(
        &self,
        bucket_name: &str,
    ) -> anyhow::Result<Vec<ObjectVersionSummary>> {
        validate_bucket_name(bucket_name)?;
        let rows = query(
            r#"
            SELECT o.object_key, v.s3_version_id, v.is_delete_marker,
                (o.current_version_id = v.id) AS is_latest,
                v.size_bytes, v.object_hash, v.created_at, v.created_by
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.object_id = o.id
            WHERE b.name = $1
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
            ORDER BY o.object_key ASC, v.created_at DESC
            "#,
        )
        .bind(bucket_name)
        .fetch_all(&self.pool)
        .await
        .context("failed to list object versions")?;
        Ok(rows
            .into_iter()
            .map(|row| ObjectVersionSummary {
                key: row.get("object_key"),
                version_id: row.get("s3_version_id"),
                is_latest: row.get("is_latest"),
                is_delete_marker: row.get("is_delete_marker"),
                size_bytes: row.get("size_bytes"),
                sha256: row.get("object_hash"),
                last_modified: format_datetime(row.get("created_at")),
                created_by: row.try_get("created_by").ok().flatten(),
            })
            .collect())
    }

    pub async fn update_s3_lifecycle_rules(
        &self,
        bucket_name: &str,
        rules: serde_json::Value,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_lifecycle_rules(&rules)?;
        query(
            r#"
            UPDATE bucket_policies p
            SET s3_lifecycle_rules = $2, updated_at = now()
            FROM buckets b
            WHERE b.id = p.bucket_id
              AND b.name = $1
              AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(rules)
        .execute(&self.pool)
        .await
        .context("failed to update S3 lifecycle rules")?;
        Ok(())
    }

    pub async fn update_s3_encryption(
        &self,
        bucket_name: &str,
        algorithm: &str,
        key_id: Option<&str>,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_policy_enum(
            "s3DefaultEncryptionAlgorithm",
            algorithm,
            &["NONE", "AES256", "aws:kms"],
        )?;
        query(
            r#"
            UPDATE bucket_policies p
            SET s3_default_encryption_algorithm = $2,
                s3_default_encryption_key_id = $3,
                updated_at = now()
            FROM buckets b
            WHERE b.id = p.bucket_id
              AND b.name = $1
              AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(algorithm)
        .bind(key_id)
        .execute(&self.pool)
        .await
        .context("failed to update S3 encryption config")?;
        Ok(())
    }

    pub async fn update_s3_object_lock_config(
        &self,
        bucket_name: &str,
        enabled: bool,
        mode: Option<&str>,
        retain_days: Option<i64>,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        if let Some(mode) = mode {
            validate_policy_enum("objectLockMode", mode, &["GOVERNANCE", "COMPLIANCE"])?;
        }
        if let Some(days) = retain_days {
            if days < 1 {
                bail!("object lock default retention days must be positive");
            }
        }
        query(
            r#"
            UPDATE bucket_policies p
            SET s3_object_lock_enabled = $2,
                s3_object_lock_default_mode = $3,
                s3_object_lock_default_retain_days = $4,
                s3_versioning_enabled = CASE WHEN $2 THEN TRUE ELSE s3_versioning_enabled END,
                updated_at = now()
            FROM buckets b
            WHERE b.id = p.bucket_id
              AND b.name = $1
              AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(enabled)
        .bind(mode)
        .bind(retain_days)
        .execute(&self.pool)
        .await
        .context("failed to update S3 object lock config")?;
        Ok(())
    }

    pub async fn update_s3_resource_policy(
        &self,
        bucket_name: &str,
        policy: serde_json::Value,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_s3_resource_policy(&policy)?;
        query(
            r#"
            UPDATE bucket_policies p
            SET s3_resource_policy = $2, updated_at = now()
            FROM buckets b
            WHERE b.id = p.bucket_id
              AND b.name = $1
              AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(policy)
        .execute(&self.pool)
        .await
        .context("failed to update S3 bucket resource policy")?;
        Ok(())
    }

    pub async fn update_s3_event_notifications(
        &self,
        bucket_name: &str,
        config: serde_json::Value,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        query(
            r#"
            UPDATE bucket_policies p
            SET s3_event_notifications = $2, updated_at = now()
            FROM buckets b
            WHERE b.id = p.bucket_id
              AND b.name = $1
              AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(config)
        .execute(&self.pool)
        .await
        .context("failed to update S3 event notification config")?;
        Ok(())
    }

    pub async fn update_object_retention(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
        mode: &str,
        retain_until: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        validate_policy_enum("objectLockMode", mode, &["GOVERNANCE", "COMPLIANCE"])?;
        self.update_current_or_version_lock(
            bucket_name,
            object_key,
            version_id,
            Some(mode),
            Some(retain_until),
            None,
        )
        .await
    }

    pub async fn update_object_legal_hold(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
        enabled: bool,
    ) -> anyhow::Result<()> {
        self.update_current_or_version_lock(
            bucket_name,
            object_key,
            version_id,
            None,
            None,
            Some(enabled),
        )
        .await
    }

    async fn update_current_or_version_lock(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
        mode: Option<&str>,
        retain_until: Option<chrono::DateTime<chrono::Utc>>,
        legal_hold: Option<bool>,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let result = query(
            r#"
            UPDATE object_versions v
            SET object_lock_mode = COALESCE($4, object_lock_mode),
                retain_until = COALESCE($5, retain_until),
                legal_hold = COALESCE($6, legal_hold)
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            WHERE v.object_id = o.id
              AND b.name = $1
              AND o.object_key = $2
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND (($3::text IS NULL AND v.id = o.current_version_id) OR v.s3_version_id = $3)
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .bind(version_id)
        .bind(mode)
        .bind(retain_until)
        .bind(legal_hold)
        .execute(&self.pool)
        .await
        .context("failed to update object lock metadata")?;
        if result.rows_affected() == 0 {
            bail!("object not found: {object_key}");
        }
        Ok(())
    }

    pub async fn record_s3_notification_event(
        &self,
        bucket_name: &str,
        object_key: &str,
        version_id: Option<&str>,
        event_name: &str,
        detail: serde_json::Value,
    ) -> anyhow::Result<()> {
        let policy = self.get_bucket_policy(bucket_name).await?;
        if !notifications_include_event(&policy.s3_event_notifications, event_name) {
            return Ok(());
        }
        query(
            r#"
            INSERT INTO s3_notification_events (bucket_id, object_id, event_name, object_key, version_id, detail)
            SELECT b.id, o.id, $3, $2, $4, $5
            FROM buckets b
            LEFT JOIN objects o ON o.bucket_id = b.id AND o.object_key = $2 AND o.deleted_at IS NULL
            WHERE b.name = $1 AND b.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .bind(event_name)
        .bind(version_id)
        .bind(detail)
        .execute(&self.pool)
        .await
        .context("failed to record S3 notification event")?;
        Ok(())
    }

    pub async fn apply_s3_lifecycle(&self, bucket_name: &str) -> anyhow::Result<S3LifecycleResult> {
        let policy = self.get_bucket_policy(bucket_name).await?;
        let rules = lifecycle_rules(&policy.s3_lifecycle_rules)?;
        let mut expired_objects = 0_i64;
        let mut aborted_multipart_uploads = 0_i64;
        for rule in rules {
            if let Some(days) = rule.expire_current_days {
                let rows = query(
                    r#"
                    SELECT o.object_key
                    FROM objects o
                    JOIN buckets b ON b.id = o.bucket_id
                    JOIN object_versions v ON v.id = o.current_version_id
                    WHERE b.name = $1
                      AND b.deleted_at IS NULL
                      AND o.deleted_at IS NULL
                      AND o.state = 'AVAILABLE'
                      AND o.object_key LIKE $2 || '%'
                      AND v.created_at <= now() - ($3::bigint * interval '1 day')
                    "#,
                )
                .bind(bucket_name)
                .bind(&rule.prefix)
                .bind(days)
                .fetch_all(&self.pool)
                .await
                .context("failed to find lifecycle-expired objects")?;
                for row in rows {
                    let key: String = row.get("object_key");
                    if self.delete_object(bucket_name, &key).await.is_ok() {
                        expired_objects += 1;
                        self.record_s3_notification_event(
                            bucket_name,
                            &key,
                            None,
                            "s3:LifecycleExpiration:Delete",
                            serde_json::json!({"reason":"Lifecycle Expiration"}),
                        )
                        .await?;
                    }
                }
            }
            if let Some(days) = rule.abort_incomplete_multipart_days {
                let result = query(
                    r#"
                    UPDATE s3_multipart_uploads u
                    SET aborted_at = now()
                    FROM buckets b
                    WHERE b.id = u.bucket_id
                      AND b.name = $1
                      AND b.deleted_at IS NULL
                      AND u.completed_at IS NULL
                      AND u.aborted_at IS NULL
                      AND u.object_key LIKE $2 || '%'
                      AND u.initiated_at <= now() - ($3::bigint * interval '1 day')
                    "#,
                )
                .bind(bucket_name)
                .bind(&rule.prefix)
                .bind(days)
                .execute(&self.pool)
                .await
                .context("failed to abort stale multipart uploads")?;
                aborted_multipart_uploads +=
                    i64::try_from(result.rows_affected()).unwrap_or(i64::MAX);
            }
        }
        Ok(S3LifecycleResult {
            expired_objects,
            aborted_multipart_uploads,
        })
    }

    pub async fn totals(&self) -> anyhow::Result<ObjectTotals> {
        let row = query(
            r#"
            SELECT
                (SELECT COUNT(*)::bigint FROM buckets WHERE deleted_at IS NULL) AS total_buckets,
                COUNT(o.id)::bigint AS total_objects,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_object_bytes
            FROM objects o
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE o.deleted_at IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to count objects")?;

        Ok(ObjectTotals {
            total_buckets: row.get("total_buckets"),
            total_objects: row.get("total_objects"),
            total_object_bytes: row.get("total_object_bytes"),
        })
    }

    pub async fn create_application_credential(
        &self,
        name: &str,
        scopes: Vec<String>,
    ) -> anyhow::Result<CreatedApplicationCredential> {
        let name = name.trim();
        validate_credential_name(name, "application name")?;
        let scopes = validate_application_scopes(&scopes)?;

        let token = secure_url_token("pm_app_", 32);
        let token_hash = hash_bearer_token(&token);
        let scopes_json = serde_json::to_value(&scopes).context("failed to serialize scopes")?;
        let row = query(
            r#"
            INSERT INTO application_credentials (name, token_hash, scopes)
            VALUES ($1, $2, $3)
            RETURNING id::text, name, scopes, created_at, revoked_at
            "#,
        )
        .bind(name)
        .bind(token_hash)
        .bind(scopes_json)
        .fetch_one(&self.pool)
        .await
        .context("failed to create application credential")?;

        Ok(CreatedApplicationCredential {
            credential: application_summary_from_row(row)?,
            token,
        })
    }

    pub async fn list_application_credentials(
        &self,
    ) -> anyhow::Result<Vec<ApplicationCredentialSummary>> {
        let rows = query(
            r#"
            SELECT id::text, name, scopes, created_at, revoked_at
            FROM application_credentials
            ORDER BY created_at DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list application credentials")?;

        rows.into_iter().map(application_summary_from_row).collect()
    }

    pub async fn revoke_application_credential(&self, id: &str) -> anyhow::Result<()> {
        let result = query(
            "UPDATE application_credentials SET revoked_at = now() WHERE id = $1::uuid AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("failed to revoke application credential")?;
        if result.rows_affected() == 0 {
            bail!("application credential not found or already revoked: {id}");
        }
        Ok(())
    }

    pub async fn get_mcp_settings(&self) -> anyhow::Result<McpSettings> {
        let row = query(
            r#"
            INSERT INTO mcp_settings (id)
            VALUES (TRUE)
            ON CONFLICT (id) DO UPDATE SET id = EXCLUDED.id
            RETURNING enabled, endpoint_path, bind_host, require_auth,
                read_tools_enabled, write_tools_enabled, admin_tools_enabled, expose_resources,
                expose_prompts, allow_localhost_only, created_at, updated_at
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to load MCP settings")?;
        Ok(mcp_settings_from_row(row))
    }

    pub async fn update_mcp_settings(
        &self,
        update: McpSettingsUpdate,
    ) -> anyhow::Result<McpSettings> {
        validate_mcp_settings_update(&update)?;
        let row = query(
            r#"
            INSERT INTO mcp_settings (
                id, enabled, endpoint_path, bind_host, require_auth,
                read_tools_enabled, write_tools_enabled, admin_tools_enabled, expose_resources,
                expose_prompts, allow_localhost_only, updated_at
            )
            VALUES (TRUE, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
            ON CONFLICT (id) DO UPDATE
            SET enabled = EXCLUDED.enabled,
                endpoint_path = EXCLUDED.endpoint_path,
                bind_host = EXCLUDED.bind_host,
                require_auth = EXCLUDED.require_auth,
                read_tools_enabled = EXCLUDED.read_tools_enabled,
                write_tools_enabled = EXCLUDED.write_tools_enabled,
                admin_tools_enabled = EXCLUDED.admin_tools_enabled,
                expose_resources = EXCLUDED.expose_resources,
                expose_prompts = EXCLUDED.expose_prompts,
                allow_localhost_only = EXCLUDED.allow_localhost_only,
                updated_at = now()
            RETURNING enabled, endpoint_path, bind_host, require_auth,
                read_tools_enabled, write_tools_enabled, admin_tools_enabled, expose_resources,
                expose_prompts, allow_localhost_only, created_at, updated_at
            "#,
        )
        .bind(update.enabled)
        .bind(&update.endpoint_path)
        .bind(update.bind_host.as_deref())
        .bind(update.require_auth)
        .bind(update.read_tools_enabled)
        .bind(update.write_tools_enabled)
        .bind(update.admin_tools_enabled)
        .bind(update.expose_resources)
        .bind(update.expose_prompts)
        .bind(update.allow_localhost_only)
        .fetch_one(&self.pool)
        .await
        .context("failed to update MCP settings")?;
        Ok(mcp_settings_from_row(row))
    }

    pub async fn mcp_status(&self) -> anyhow::Result<McpStatus> {
        let settings = self.get_mcp_settings().await?;
        let row = query(
            r#"
            SELECT
                (SELECT MAX(created_at) FROM mcp_activity_events) AS last_activity_at,
                (SELECT COUNT(*)::bigint FROM mcp_activity_events WHERE created_at > now() - interval '24 hours') AS recent_calls_count
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to load MCP status")?;
        Ok(McpStatus {
            enabled: settings.enabled,
            endpoint: settings.endpoint_path,
            auth_required: settings.require_auth,
            read_tools_enabled: settings.read_tools_enabled,
            write_tools_enabled: settings.write_tools_enabled,
            admin_tools_enabled: settings.admin_tools_enabled,
            resources_enabled: settings.expose_resources,
            prompts_enabled: settings.expose_prompts,
            last_activity_at: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_activity_at")
                .map(format_datetime),
            active_sessions_count: 0,
            recent_calls_count: row.get("recent_calls_count"),
        })
    }

    pub async fn create_mcp_access_token(
        &self,
        name: &str,
        scopes: &[String],
        created_by_user_id: Option<&str>,
    ) -> anyhow::Result<CreatedMcpAccessToken> {
        let name = name.trim();
        validate_credential_name(name, "MCP token name")?;
        let scopes = validate_mcp_scopes(scopes)?;
        let secret = secure_url_token("pm_mcp_", 32);
        let token_prefix = secret.chars().take(14).collect::<String>();
        let token_hash = hash_bearer_token(&secret);
        let row = query(
            r#"
            INSERT INTO mcp_access_tokens (name, token_prefix, token_hash, scopes, created_by_user_id)
            VALUES ($1, $2, $3, $4, $5::uuid)
            RETURNING id::text, name, token_prefix, is_active, created_at, revoked_at, last_used_at, scopes
            "#,
        )
        .bind(name)
        .bind(&token_prefix)
        .bind(token_hash)
        .bind(&scopes)
        .bind(created_by_user_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to create MCP access token")?;
        Ok(CreatedMcpAccessToken {
            token: mcp_access_token_from_row(row),
            secret,
        })
    }

    pub async fn list_mcp_access_tokens(&self) -> anyhow::Result<Vec<McpAccessTokenSummary>> {
        let rows = query(
            r#"
            SELECT id::text, name, token_prefix, is_active, created_at, revoked_at, last_used_at, scopes
            FROM mcp_access_tokens
            ORDER BY created_at DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list MCP access tokens")?;
        Ok(rows.into_iter().map(mcp_access_token_from_row).collect())
    }

    pub async fn revoke_mcp_access_token(&self, id: &str) -> anyhow::Result<()> {
        let result = query(
            "UPDATE mcp_access_tokens SET is_active = FALSE, revoked_at = now() WHERE id = $1::uuid AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("failed to revoke MCP access token")?;
        if result.rows_affected() == 0 {
            bail!("MCP token not found or already revoked: {id}");
        }
        Ok(())
    }

    pub async fn authorize_mcp_token(
        &self,
        token: &str,
    ) -> anyhow::Result<Option<McpTokenAuthorization>> {
        let token_hash = hash_bearer_token(token);
        let row = query(
            r#"
            SELECT id::text, name, scopes
            FROM mcp_access_tokens
            WHERE token_hash = $1
              AND is_active = TRUE
              AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to authorize MCP token")?;
        Ok(row.map(|row| McpTokenAuthorization {
            id: row.get("id"),
            name: row.get("name"),
            scopes: row.get("scopes"),
        }))
    }

    pub async fn record_mcp_token_used(&self, token_id: &str) -> anyhow::Result<()> {
        query("UPDATE mcp_access_tokens SET last_used_at = now() WHERE id = $1::uuid")
            .bind(token_id)
            .execute(&self.pool)
            .await
            .context("failed to update MCP token usage")?;
        Ok(())
    }

    pub async fn count_recent_mcp_activity(
        &self,
        token_id: &str,
        window_seconds: i64,
    ) -> anyhow::Result<i64> {
        let count: i64 = sqlx_core::query_scalar::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM mcp_activity_events
            WHERE token_id = $1::uuid
              AND created_at >= now() - ($2::bigint * INTERVAL '1 second')
            "#,
        )
        .bind(token_id)
        .bind(window_seconds.max(1))
        .fetch_one(&self.pool)
        .await
        .context("failed to count recent MCP activity")?;
        Ok(count)
    }

    pub async fn record_mcp_activity(
        &self,
        token_id: Option<&str>,
        method: &str,
        target: Option<&str>,
        outcome: &str,
        detail: serde_json::Value,
    ) -> anyhow::Result<()> {
        query(
            r#"
            INSERT INTO mcp_activity_events (token_id, method, target, outcome, detail)
            VALUES ($1::uuid, $2, $3, $4, $5)
            "#,
        )
        .bind(token_id)
        .bind(method)
        .bind(target)
        .bind(outcome)
        .bind(detail)
        .execute(&self.pool)
        .await
        .context("failed to record MCP activity")?;
        self.record_audit_event(
            mcp_audit_event_for_method(method, outcome),
            token_id,
            outcome,
            &format!("method={method}; target={}", target.unwrap_or("")),
        )
        .await?;
        Ok(())
    }

    pub async fn list_mcp_activity(&self, limit: i64) -> anyhow::Result<Vec<McpActivityRecord>> {
        let limit = limit.clamp(1, 100);
        let rows = query(
            r#"
            SELECT id::text, token_id::text, method, target, outcome, created_at
            FROM mcp_activity_events
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("failed to list MCP activity")?;
        Ok(rows.into_iter().map(mcp_activity_from_row).collect())
    }

    pub async fn create_s3_access_key(
        &self,
        user_id: &str,
        name: Option<&str>,
        secret_encryption_key: &str,
    ) -> anyhow::Result<CreatedS3AccessKey> {
        if secret_encryption_key.trim().is_empty() {
            bail!("S3 secret encryption key cannot be empty");
        }
        if user_id.trim().is_empty() {
            bail!("S3 access key user id cannot be empty");
        }
        let access_key_id = secure_url_token("PMK", 18);
        let secret_access_key = secure_url_token("", 40);
        let secret_key_hash = hash_bearer_token(&secret_access_key);
        let normalized_name = name.and_then(normalize_optional_name);
        if normalized_name
            .as_deref()
            .is_some_and(|name| name.chars().count() > 255)
        {
            bail!("S3 access key name cannot exceed 255 characters");
        }
        let row = query(
            r#"
            INSERT INTO s3_access_keys (
                access_key_id, secret_key_hash, secret_key_ciphertext, user_id, name, is_active
            )
            VALUES ($1, $2, pgp_sym_encrypt($3, $4), $5::uuid, $6, TRUE)
            RETURNING id::text, name, access_key_id, user_id::text, is_active,
                      created_at, revoked_at, last_used_at
            "#,
        )
        .bind(&access_key_id)
        .bind(secret_key_hash)
        .bind(&secret_access_key)
        .bind(secret_encryption_key)
        .bind(user_id)
        .bind(normalized_name)
        .fetch_one(&self.pool)
        .await
        .context("failed to create S3 access key")?;

        let key = s3_access_key_summary_from_row(row);
        Ok(CreatedS3AccessKey {
            id: key.id,
            name: key.name,
            access_key_id: key.access_key_id,
            secret_access_key,
            created_at: key.created_at,
        })
    }

    pub async fn list_s3_access_keys(
        &self,
        page: u32,
        page_size: u32,
    ) -> anyhow::Result<PaginatedS3AccessKeys> {
        let total: i64 = query_scalar("SELECT COUNT(*) FROM s3_access_keys")
            .fetch_one(&self.pool)
            .await
            .context("failed to count S3 access keys")?;
        let total_pages = total_pages(total, page_size);
        let page = page.min(total_pages).max(1);
        let offset = i64::from(page.saturating_sub(1)) * i64::from(page_size);
        let limit = i64::from(page_size);
        let rows = query(
            r#"
            SELECT id::text, name, access_key_id, user_id::text, is_active,
                   created_at, revoked_at, last_used_at
            FROM s3_access_keys
            ORDER BY created_at DESC, access_key_id ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("failed to list S3 access keys")?;

        Ok(PaginatedS3AccessKeys {
            items: rows
                .into_iter()
                .map(s3_access_key_summary_from_row)
                .collect(),
            page,
            page_size,
            total,
            total_pages,
        })
    }

    pub async fn revoke_s3_access_key(&self, access_key_id: &str) -> anyhow::Result<()> {
        let result = query(
            r#"
            UPDATE s3_access_keys
            SET is_active = FALSE, revoked_at = now()
            WHERE access_key_id = $1
              AND revoked_at IS NULL
            "#,
        )
        .bind(access_key_id)
        .execute(&self.pool)
        .await
        .context("failed to revoke S3 access key")?;

        if result.rows_affected() == 0 {
            bail!("S3 access key not found or already revoked");
        }
        Ok(())
    }

    pub async fn revoke_s3_access_key_by_id(&self, id: &str) -> anyhow::Result<S3AccessKeySummary> {
        let row = query(
            r#"
            UPDATE s3_access_keys
            SET is_active = FALSE,
                revoked_at = COALESCE(revoked_at, now())
            WHERE id = $1::uuid
            RETURNING id::text, name, access_key_id, user_id::text, is_active,
                      created_at, revoked_at, last_used_at
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to revoke S3 access key")?;

        let Some(row) = row else {
            bail!("S3 access key not found");
        };
        Ok(s3_access_key_summary_from_row(row))
    }

    pub async fn find_application_by_token_hash(
        &self,
        token_hash: &str,
    ) -> anyhow::Result<Option<ApplicationCredential>> {
        let row = query(
            r#"
            SELECT id::text, name, scopes
            FROM application_credentials
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load application credential")?;

        row.map(|row| {
            Ok(ApplicationCredential {
                id: row.get("id"),
                name: row.get("name"),
                scopes: parse_string_vec(row.get("scopes"))?,
            })
        })
        .transpose()
    }

    pub async fn ensure_s3_access_key(
        &self,
        user_id: Option<&str>,
        name: Option<&str>,
        access_key_id: &str,
        secret_access_key: &str,
        secret_encryption_key: &str,
    ) -> anyhow::Result<S3AccessKey> {
        let access_key_id = access_key_id.trim();
        if access_key_id.is_empty() {
            bail!("S3 access key id cannot be empty");
        }
        if secret_access_key.trim().len() < 20 {
            bail!("S3 secret access key must have at least 20 characters");
        }
        if secret_encryption_key.trim().is_empty() {
            bail!("S3 secret encryption key cannot be empty");
        }
        let normalized_name = name.and_then(normalize_optional_name);
        let secret_key_hash = hash_bearer_token(secret_access_key.trim());

        let row = query(
            r#"
            INSERT INTO s3_access_keys (
                access_key_id, secret_key_hash, secret_key_ciphertext, user_id, name, is_active
            )
            VALUES ($1, $2, pgp_sym_encrypt($3, $4), $5::uuid, $6, TRUE)
            ON CONFLICT (access_key_id) DO UPDATE SET
                secret_key_hash = EXCLUDED.secret_key_hash,
                secret_key_ciphertext = EXCLUDED.secret_key_ciphertext,
                user_id = COALESCE(EXCLUDED.user_id, s3_access_keys.user_id),
                name = COALESCE(EXCLUDED.name, s3_access_keys.name),
                is_active = TRUE,
                revoked_at = NULL
            RETURNING access_key_id, secret_key_hash
            "#,
        )
        .bind(access_key_id)
        .bind(&secret_key_hash)
        .bind(secret_access_key.trim())
        .bind(secret_encryption_key)
        .bind(user_id)
        .bind(normalized_name)
        .fetch_one(&self.pool)
        .await
        .context("failed to upsert S3 access key")?;

        Ok(S3AccessKey {
            access_key_id: row.get("access_key_id"),
            secret_key_hash: row.get("secret_key_hash"),
            secret_access_key: None,
        })
    }

    pub async fn find_s3_access_key_for_signing(
        &self,
        access_key_id: &str,
        secret_encryption_key: &str,
    ) -> anyhow::Result<Option<S3AccessKey>> {
        let row = query(
            r#"
            SELECT access_key_id, secret_key_hash,
                   CASE
                       WHEN secret_key_ciphertext IS NULL THEN NULL
                       ELSE pgp_sym_decrypt(secret_key_ciphertext, $2)
                   END AS secret_access_key
            FROM s3_access_keys
            WHERE access_key_id = $1
              AND is_active = TRUE
              AND revoked_at IS NULL
            "#,
        )
        .bind(access_key_id)
        .bind(secret_encryption_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load S3 access key signing material")?;

        Ok(row.map(|row| S3AccessKey {
            access_key_id: row.get("access_key_id"),
            secret_key_hash: row.get("secret_key_hash"),
            secret_access_key: row.get("secret_access_key"),
        }))
    }

    pub async fn record_s3_access_key_used(&self, access_key_id: &str) -> anyhow::Result<()> {
        query(
            r#"
            UPDATE s3_access_keys
            SET last_used_at = now()
            WHERE access_key_id = $1
            "#,
        )
        .bind(access_key_id)
        .execute(&self.pool)
        .await
        .context("failed to update S3 access key last_used_at")?;
        Ok(())
    }

    pub async fn create_replica_credential(
        &self,
        name: &str,
        allowed_buckets: Vec<String>,
    ) -> anyhow::Result<CreatedReplicaCredential> {
        let name = name.trim();
        validate_credential_name(name, "replica name")?;
        if allowed_buckets.is_empty() {
            bail!("replica must include at least one allowed bucket");
        }
        for bucket in &allowed_buckets {
            validate_bucket_name(bucket)?;
        }

        let token = secure_url_token("pm_rep_", 32);
        let token_hash = hash_bearer_token(&token);
        let allowed_buckets_json =
            serde_json::to_value(&allowed_buckets).context("failed to serialize buckets")?;
        let row = query(
            r#"
            INSERT INTO replica_credentials (name, token_hash, allowed_buckets)
            VALUES ($1, $2, $3)
            RETURNING id::text, name, allowed_buckets, created_at, revoked_at,
                0::bigint AS available_objects,
                NULL::timestamptz AS last_seen_at,
                NULL::text AS health_status,
                NULL::timestamptz AS health_reported_at
            "#,
        )
        .bind(name)
        .bind(token_hash)
        .bind(allowed_buckets_json)
        .fetch_one(&self.pool)
        .await
        .context("failed to create replica credential")?;

        Ok(CreatedReplicaCredential {
            replica: replica_summary_from_row(row)?,
            token,
        })
    }

    pub async fn list_replicas(&self) -> anyhow::Result<Vec<ReplicaSummary>> {
        let rows = query(
            r#"
            SELECT
                r.id::text,
                r.name,
                r.allowed_buckets,
                r.created_at,
                r.revoked_at,
                COUNT(a.object_id)::bigint AS available_objects,
                MAX(a.last_seen_at) AS last_seen_at,
                h.status AS health_status,
                h.reported_at AS health_reported_at
            FROM replica_credentials r
            LEFT JOIN replica_object_availability a ON a.replica_id = r.id
            LEFT JOIN LATERAL (
                SELECT status, reported_at
                FROM replica_health_reports
                WHERE replica_id = r.id
                ORDER BY reported_at DESC
                LIMIT 1
            ) h ON TRUE
            GROUP BY r.id, r.name, r.allowed_buckets, r.created_at, r.revoked_at,
                h.status, h.reported_at
            ORDER BY r.created_at DESC, r.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list replicas")?;

        rows.into_iter().map(replica_summary_from_row).collect()
    }

    pub async fn revoke_replica(&self, replica_id: &str) -> anyhow::Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin replica revocation transaction")?;
        let target = query(
            r#"
            SELECT id, allowed_buckets
            FROM replica_credentials
            WHERE id = $1::uuid AND revoked_at IS NULL
            "#,
        )
        .bind(replica_id)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to load replica before revocation")?;
        let Some(target) = target else {
            bail!("replica not found or already revoked: {replica_id}");
        };
        let allowed_buckets = parse_string_vec(target.get("allowed_buckets"))?;
        query(
            r#"
            INSERT INTO replica_policy_updates (
                replica_id, update_type, bucket_id, object_id, detail
            )
            VALUES ($1::uuid, 'replica_revoked', NULL, NULL, $2)
            "#,
        )
        .bind(replica_id)
        .bind(serde_json::json!({
            "reason": "replica credential revoked",
            "allowedBuckets": allowed_buckets
        }))
        .execute(&mut *tx)
        .await
        .context("failed to record replica revocation policy update")?;
        let result = query(
            "UPDATE replica_credentials SET revoked_at = now() WHERE id = $1::uuid AND revoked_at IS NULL",
        )
        .bind(replica_id)
        .execute(&mut *tx)
        .await
        .context("failed to revoke replica")?;
        if result.rows_affected() == 0 {
            bail!("replica not found or already revoked: {replica_id}");
        }
        query("DELETE FROM replica_object_availability WHERE replica_id = $1::uuid")
            .bind(replica_id)
            .execute(&mut *tx)
            .await
            .context("failed to remove revoked replica availability")?;
        tx.commit()
            .await
            .context("failed to commit replica revocation")?;
        Ok(())
    }

    pub async fn find_replica_by_token_hash(
        &self,
        token_hash: &str,
    ) -> anyhow::Result<Option<ReplicaCredential>> {
        let row = query(
            r#"
            SELECT id::text, name, allowed_buckets, revoked_at IS NOT NULL AS revoked
            FROM replica_credentials
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load replica credential")?;

        row.map(|row| {
            Ok(ReplicaCredential {
                id: row.get("id"),
                name: row.get("name"),
                allowed_buckets: parse_string_vec(row.get("allowed_buckets"))?,
                revoked: row.get("revoked"),
            })
        })
        .transpose()
    }

    pub async fn record_replica_request_nonce(
        &self,
        replica_id: &str,
        nonce: &str,
    ) -> anyhow::Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin replica nonce transaction")?;
        query("DELETE FROM replica_request_nonces WHERE seen_at < now() - interval '10 minutes'")
            .execute(&mut *tx)
            .await
            .context("failed to prune replica request nonces")?;
        let result = query(
            r#"
            INSERT INTO replica_request_nonces (replica_id, nonce)
            VALUES ($1::uuid, $2)
            ON CONFLICT (replica_id, nonce) DO NOTHING
            "#,
        )
        .bind(replica_id)
        .bind(nonce)
        .execute(&mut *tx)
        .await
        .context("failed to record replica request nonce")?;
        if result.rows_affected() == 0 {
            bail!("replica request nonce has already been used");
        }
        tx.commit()
            .await
            .context("failed to commit replica nonce transaction")?;
        Ok(())
    }

    pub async fn authorize_replica_object_sync(
        &self,
        replica_id: &str,
        allowed_buckets: &[String],
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<ObjectRecord> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        if !allowed_buckets.iter().any(|bucket| bucket == bucket_name) {
            bail!("replica is not allowed to synchronize this bucket");
        }

        let row = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash, v.storage_path,
                   v.s3_version_id, v.is_delete_marker, v.checksum_sha256, v.checksum_crc32,
                   v.encryption_algorithm, v.encryption_key_id, v.encryption_nonce,
                   v.object_lock_mode, v.retain_until, v.legal_hold, v.created_at, o.state
            FROM replica_credentials r
            JOIN buckets b ON b.name = $2
            JOIN objects o ON o.bucket_id = b.id AND o.object_key = $3
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN bucket_policies p ON p.bucket_id = b.id
            WHERE r.id = $1::uuid
              AND r.revoked_at IS NULL
              AND r.allowed_buckets ? $2
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
            "#,
        )
        .bind(replica_id)
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to authorize replica object synchronization")?;

        row.map(object_record_from_row)
            .ok_or_else(|| anyhow::anyhow!("replica object synchronization is not authorized"))
    }

    pub async fn authorize_replica_fragment_sync(
        &self,
        replica_id: &str,
        allowed_buckets: &[String],
        manifest_id: &str,
        fragment_id: &str,
    ) -> anyhow::Result<AuthorizedReplicaFragment> {
        let (fragment_manifest_id, fragment_index, fragment_hash) = parse_fragment_id(fragment_id)?;
        if fragment_manifest_id != manifest_id {
            bail!("fragment does not belong to requested manifest");
        }

        let row = query(
            r#"
            SELECT b.name AS bucket_name, o.object_key, v.size_bytes, v.content_type,
                   v.object_hash, v.storage_path, v.s3_version_id, v.created_at, o.state,
                   m.id::text AS manifest_id,
                   f.fragment_index, f.fragment_hash,
                   f.byte_range_start, f.byte_range_end
            FROM object_manifests m
            JOIN object_manifest_fragments f ON f.manifest_id = m.id
            JOIN object_versions v ON v.id = m.object_version_id
            JOIN objects o ON o.current_version_id = v.id
            JOIN buckets b ON b.id = o.bucket_id
            JOIN bucket_policies p ON p.bucket_id = b.id
            JOIN replica_credentials r ON r.id = $1::uuid
            WHERE m.id = $2::uuid
              AND f.fragment_index = $3
              AND f.fragment_hash = $4
              AND r.revoked_at IS NULL
              AND r.allowed_buckets ? b.name
              AND b.name = ANY($5)
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
            "#,
        )
        .bind(replica_id)
        .bind(manifest_id)
        .bind(fragment_index)
        .bind(&fragment_hash)
        .bind(allowed_buckets)
        .fetch_optional(&self.pool)
        .await
        .context("failed to authorize replica fragment synchronization")?;

        let Some(row) = row else {
            bail!("replica fragment synchronization is not authorized");
        };

        Ok(AuthorizedReplicaFragment {
            object: ObjectRecord {
                key: row.get("object_key"),
                size_bytes: row.get("size_bytes"),
                content_type: row
                    .get::<Option<String>, _>("content_type")
                    .unwrap_or_else(|| "application/octet-stream".to_owned()),
                sha256: row.get("object_hash"),
                storage_path: row.get("storage_path"),
                version_id: row.get("s3_version_id"),
                is_delete_marker: false,
                checksum_sha256: row.get("object_hash"),
                checksum_crc32: None,
                encryption_algorithm: None,
                encryption_key_id: None,
                encryption_nonce: None,
                object_lock_mode: None,
                retain_until: None,
                legal_hold: false,
                created_at: format_datetime(row.get("created_at")),
                state: row.get("state"),
                user_metadata: None,
                created_by: None,
            },
            bucket_name: row.get("bucket_name"),
            object_key: row.get("object_key"),
            manifest_id: row.get("manifest_id"),
            fragment_index: row.get("fragment_index"),
            fragment_hash: row.get("fragment_hash"),
            byte_range_start: row.get("byte_range_start"),
            byte_range_end: row.get("byte_range_end"),
        })
    }

    pub async fn list_replica_sync_objects(
        &self,
        allowed_buckets: &[String],
    ) -> anyhow::Result<Vec<ReplicaSyncObject>> {
        for bucket in allowed_buckets {
            validate_bucket_name(bucket)?;
        }
        let rows = query(
            r#"
            SELECT b.name, o.object_key, v.size_bytes, v.content_type, v.object_hash, o.state,
                   m.id::text AS manifest_id,
                   f.fragment_index, f.byte_range_start, f.byte_range_end,
                   f.size_bytes AS fragment_size_bytes, f.fragment_hash
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            JOIN object_manifest_fragments f ON f.manifest_id = m.id
            JOIN bucket_policies p ON p.bucket_id = b.id
            WHERE b.name = ANY($1)
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
            ORDER BY v.created_at DESC, o.object_key ASC, f.fragment_index ASC
            "#,
        )
        .bind(allowed_buckets)
        .fetch_all(&self.pool)
        .await
        .context("failed to list replica sync objects")?;

        let mut objects = Vec::<ReplicaSyncObject>::new();
        for row in rows {
            let manifest_id: String = row.get("manifest_id");
            let object = match objects
                .iter_mut()
                .find(|object| object.manifest_id == manifest_id)
            {
                Some(object) => object,
                None => {
                    objects.push(ReplicaSyncObject {
                        bucket: row.get("name"),
                        key: row.get("object_key"),
                        manifest_id: manifest_id.clone(),
                        size_bytes: row.get("size_bytes"),
                        content_type: row.get("content_type"),
                        sha256: row.get("object_hash"),
                        state: row.get("state"),
                        election_epoch: format!("{}:{manifest_id}", row.get::<String, _>("state")),
                        election_leader_id: None,
                        replica_set: Vec::new(),
                        fragments: Vec::new(),
                    });
                    objects.last_mut().expect("object was just pushed")
                }
            };
            let index = row.get("fragment_index");
            let sha256 = row.get("fragment_hash");
            object.fragments.push(ReplicaSyncFragment {
                index,
                fragment_id: format!("{manifest_id}:{index}:{sha256}"),
                byte_range_start: row.get("byte_range_start"),
                byte_range_end: row.get("byte_range_end"),
                size_bytes: row.get("fragment_size_bytes"),
                sha256,
            });
        }

        for object in &mut objects {
            object.replica_set = self
                .list_replica_sync_members(&object.bucket, &object.key)
                .await?;
            object.election_leader_id = elected_replica_leader(&object.replica_set);
        }

        Ok(objects)
    }

    async fn list_replica_sync_members(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Vec<ReplicaSyncMember>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let rows = query(
            r#"
            SELECT
                r.id::text AS replica_id,
                r.name AS replica_name,
                a.endpoint,
                a.last_seen_at
            FROM replica_credentials r
            JOIN buckets b ON b.name = $1
            JOIN objects o ON o.bucket_id = b.id AND o.object_key = $2
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            JOIN bucket_policies p ON p.bucket_id = b.id
            LEFT JOIN replica_object_availability a
              ON a.replica_id = r.id
             AND a.bucket_id = b.id
             AND a.object_id = o.id
             AND a.object_manifest_id = m.id
            WHERE r.revoked_at IS NULL
              AND r.allowed_buckets ? b.name
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
            ORDER BY r.id ASC
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list replica election members")?;

        Ok(rows
            .into_iter()
            .map(|row| ReplicaSyncMember {
                replica_id: row.get("replica_id"),
                replica_name: row.get("replica_name"),
                endpoint: row.get("endpoint"),
                last_seen_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_seen_at")
                    .map(format_datetime),
            })
            .collect())
    }

    pub async fn record_replica_object_availability(
        &self,
        replica_id: &str,
        allowed_buckets: &[String],
        bucket_name: &str,
        object_key: &str,
        endpoint: &str,
        available_fragments: &[i64],
    ) -> anyhow::Result<ReplicaAvailabilityRecord> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        if !allowed_buckets.iter().any(|bucket| bucket == bucket_name) {
            bail!("replica is not allowed to announce this bucket");
        }
        validate_replica_endpoint(endpoint)?;
        for fragment in available_fragments {
            if *fragment < 0 {
                bail!("available fragment indexes must be non-negative");
            }
        }

        let available_fragments_json =
            serde_json::to_value(available_fragments).context("failed to serialize fragments")?;
        let row = query(
            r#"
            WITH target AS (
                SELECT
                    r.id AS replica_id,
                    r.name AS replica_name,
                    b.id AS bucket_id,
                    b.name AS bucket_name,
                    o.id AS object_id,
                    o.object_key,
                    m.id AS manifest_id
                FROM replica_credentials r
                JOIN buckets b ON b.name = $2
                JOIN objects o ON o.bucket_id = b.id AND o.object_key = $3
                JOIN object_versions v ON v.id = o.current_version_id
                JOIN object_manifests m ON m.object_version_id = v.id
                JOIN bucket_policies p ON p.bucket_id = b.id
                WHERE r.id = $1::uuid
                  AND r.revoked_at IS NULL
                  AND r.allowed_buckets ? $2
                  AND b.deleted_at IS NULL
                  AND o.deleted_at IS NULL
                  AND o.state = 'AVAILABLE'
                  AND p.allow_replica_edge = TRUE
            )
            INSERT INTO replica_object_availability (
                replica_id, bucket_id, object_id, object_manifest_id,
                endpoint, available_fragments, last_seen_at
            )
            SELECT replica_id, bucket_id, object_id, manifest_id, $4, $5, now()
            FROM target
            ON CONFLICT (replica_id, object_manifest_id) DO UPDATE
            SET endpoint = EXCLUDED.endpoint,
                available_fragments = EXCLUDED.available_fragments,
                last_seen_at = now()
            RETURNING
                replica_id::text,
                (SELECT replica_name FROM target) AS replica_name,
                (SELECT bucket_name FROM target) AS bucket_name,
                (SELECT object_key FROM target) AS object_key,
                endpoint,
                available_fragments,
                last_seen_at
            "#,
        )
        .bind(replica_id)
        .bind(bucket_name)
        .bind(object_key)
        .bind(endpoint)
        .bind(available_fragments_json)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record replica object availability")?;

        let Some(row) = row else {
            bail!("replica availability target is not authorized or not available");
        };

        availability_record_from_row(row)
    }

    pub async fn list_authorized_replica_sources(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Vec<ReplicaAvailabilityRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let rows = query(
            r#"
            WITH replica_quality AS (
                SELECT
                    replica_id,
                    COALESCE(SUM(sync_failures + auth_failures), 0)::bigint AS recent_failures,
                    AVG(avg_latency_ms)::float8 AS avg_latency_ms
                FROM replica_metric_events
                WHERE reported_at > now() - interval '30 minutes'
                GROUP BY replica_id
            )
            SELECT
                r.id::text AS replica_id,
                r.name AS replica_name,
                b.name AS bucket_name,
                o.object_key,
                a.endpoint,
                a.available_fragments,
                a.last_seen_at
            FROM replica_object_availability a
            JOIN replica_credentials r ON r.id = a.replica_id
            JOIN buckets b ON b.id = a.bucket_id
            JOIN objects o ON o.id = a.object_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            JOIN bucket_policies p ON p.bucket_id = b.id
            LEFT JOIN replica_quality q ON q.replica_id = r.id
            WHERE b.name = $1
              AND o.object_key = $2
              AND a.object_manifest_id = m.id
              AND r.revoked_at IS NULL
              AND r.allowed_buckets ? b.name
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
              AND a.last_seen_at > now() - interval '10 minutes'
            ORDER BY COALESCE(q.recent_failures, 0) ASC,
                     q.avg_latency_ms ASC NULLS LAST,
                     a.last_seen_at DESC,
                     r.name ASC
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list authorized replica sources")?;

        rows.into_iter().map(availability_record_from_row).collect()
    }

    pub async fn record_peer_fragment_availability(
        &self,
        authorization: &AccessPackageAuthorization,
        input: PeerAvailabilityInput,
    ) -> anyhow::Result<PeerAvailabilityRecord> {
        validate_bucket_name(&authorization.bucket_name)?;
        validate_object_key(&authorization.object_key)?;
        validate_peer_id(&input.peer_id)?;
        validate_peer_endpoint(&input.endpoint)?;
        if !(30..=900).contains(&input.ttl_seconds) {
            bail!("ttlSeconds must be between 30 and 900");
        }
        if input.available_fragments.is_empty() {
            bail!("availableFragments must include at least one fragment");
        }
        for fragment in &input.available_fragments {
            if *fragment < 0 {
                bail!("available fragment indexes must be non-negative");
            }
        }

        let fragments_json = serde_json::to_value(&input.available_fragments)
            .context("failed to serialize peer fragments")?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(input.ttl_seconds);
        let row = query(
            r#"
            WITH target AS (
                SELECT
                    ap.id AS access_package_id,
                    ap.application_id,
                    b.id AS bucket_id,
                    b.name AS bucket_name,
                    o.id AS object_id,
                    o.object_key,
                    m.id AS manifest_id
                FROM access_packages ap
                JOIN application_credentials ac ON ac.id = ap.application_id
                JOIN buckets b ON b.id = ap.bucket_id
                JOIN objects o ON o.id = ap.object_id
                JOIN object_versions v ON v.id = o.current_version_id
                JOIN object_manifests m ON m.object_version_id = v.id
                JOIN bucket_policies p ON p.bucket_id = b.id
                WHERE ap.id = $1::uuid
                  AND ap.application_id = $2::uuid
                  AND b.name = $3
                  AND o.object_key = $4
                  AND ap.object_manifest_id = m.id
                  AND ap.expires_at > now()
                  AND ap.revoked_at IS NULL
                  AND ac.revoked_at IS NULL
                  AND b.deleted_at IS NULL
                  AND o.deleted_at IS NULL
                  AND o.state = 'AVAILABLE'
                  AND p.allow_peer_sharing = TRUE
                  AND NOT EXISTS (
                      SELECT 1
                      FROM jsonb_array_elements_text($7::jsonb) announced(fragment_index)
                      LEFT JOIN object_manifest_fragments mf
                        ON mf.manifest_id = m.id
                       AND mf.fragment_index = announced.fragment_index::bigint
                      WHERE mf.id IS NULL
                  )
            )
            INSERT INTO peer_fragment_availability (
                access_package_id, application_id, bucket_id, object_id, object_manifest_id,
                peer_id, endpoint, available_fragments, expires_at, last_seen_at
            )
            SELECT access_package_id, application_id, bucket_id, object_id, manifest_id,
                $5, $6, $7, $8, now()
            FROM target
            ON CONFLICT (access_package_id, peer_id, object_manifest_id) DO UPDATE
            SET endpoint = EXCLUDED.endpoint,
                available_fragments = EXCLUDED.available_fragments,
                expires_at = EXCLUDED.expires_at,
                last_seen_at = now()
            RETURNING id::text, peer_id,
                (SELECT bucket_name FROM target) AS bucket_name,
                (SELECT object_key FROM target) AS object_key,
                endpoint, available_fragments, expires_at, last_seen_at
            "#,
        )
        .bind(&authorization.package_id)
        .bind(&authorization.application_id)
        .bind(&authorization.bucket_name)
        .bind(&authorization.object_key)
        .bind(&input.peer_id)
        .bind(&input.endpoint)
        .bind(fragments_json)
        .bind(expires_at)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record peer fragment availability")?;

        let Some(row) = row else {
            bail!("peer availability is not authorized by package, policy or manifest");
        };
        peer_availability_record_from_row(row)
    }

    pub async fn list_authorized_peer_sources(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Vec<PeerAvailabilityRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let rows = query(
            r#"
            WITH peer_quality AS (
                SELECT
                    peer_id,
                    SUM(CASE WHEN outcome <> 'SUCCESS' THEN 1 ELSE 0 END)::bigint AS recent_failures,
                    AVG(latency_ms)::float8 AS avg_latency_ms
                FROM fragment_transfer_events
                WHERE peer_id IS NOT NULL
                  AND created_at > now() - interval '30 minutes'
                GROUP BY peer_id
            )
            SELECT
                pfa.id::text,
                pfa.peer_id,
                b.name AS bucket_name,
                o.object_key,
                pfa.endpoint,
                pfa.available_fragments,
                pfa.expires_at,
                pfa.last_seen_at
            FROM peer_fragment_availability pfa
            JOIN access_packages ap ON ap.id = pfa.access_package_id
            JOIN application_credentials ac ON ac.id = pfa.application_id
            JOIN buckets b ON b.id = pfa.bucket_id
            JOIN objects o ON o.id = pfa.object_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            JOIN bucket_policies bp ON bp.bucket_id = b.id
            LEFT JOIN peer_quality q ON q.peer_id = pfa.id
            WHERE b.name = $1
              AND o.object_key = $2
              AND pfa.object_manifest_id = m.id
              AND pfa.expires_at > now()
              AND ap.expires_at > now()
              AND ap.revoked_at IS NULL
              AND ac.revoked_at IS NULL
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND bp.allow_peer_sharing = TRUE
            ORDER BY COALESCE(q.recent_failures, 0) ASC,
                     q.avg_latency_ms ASC NULLS LAST,
                     pfa.last_seen_at DESC,
                     pfa.peer_id ASC
            LIMIT 50
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list authorized peer sources")?;

        rows.into_iter()
            .map(peer_availability_record_from_row)
            .collect()
    }

    pub async fn record_sdk_fragment_event(
        &self,
        authorization: &AccessPackageAuthorization,
        input: SdkFragmentEventInput,
    ) -> anyhow::Result<SdkFragmentEventRecord> {
        validate_source_type(&input.source_type)?;
        validate_sdk_event_type(&input.event_type)?;
        validate_sdk_outcome(&input.outcome)?;
        if input.source_type == "PEER" && input.peer_availability_id.is_none() {
            bail!("peerAvailabilityId is required for PEER events");
        }
        if input.source_type != "PEER" && input.peer_availability_id.is_some() {
            bail!("peerAvailabilityId is only valid for PEER events");
        }
        validate_non_negative(input.bytes_transferred, "bytesTransferred")?;
        if input.fragment_index < 0 {
            bail!("fragmentIndex must be non-negative");
        }
        if input.fragment_hash.len() != 64
            || !input.fragment_hash.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            bail!("fragmentHash must be a SHA-256 hex digest");
        }
        if let Some(value) = input.latency_ms {
            validate_non_negative(value, "latencyMs")?;
        }

        let expected_hash: Option<String> = query_scalar(
            r#"
            SELECT fragment_hash
            FROM object_manifest_fragments
            WHERE manifest_id = $1::uuid AND fragment_index = $2
            "#,
        )
        .bind(&authorization.manifest_id)
        .bind(input.fragment_index)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load expected fragment hash")?;
        let Some(expected_hash) = expected_hash else {
            bail!("fragment does not belong to the access package manifest");
        };
        let hash_matches = expected_hash.eq_ignore_ascii_case(&input.fragment_hash);
        let persisted_outcome = if hash_matches {
            input.outcome.clone()
        } else {
            "REJECTED".to_owned()
        };
        let persisted_event_type = if hash_matches {
            input.event_type.clone()
        } else {
            "HASH_MISMATCH".to_owned()
        };
        let detail = if hash_matches {
            input.detail
        } else {
            serde_json::json!({
                "expectedHash": expected_hash,
                "reportedHash": input.fragment_hash,
                "detail": input.detail
            })
        };

        let row = query(
            r#"
            WITH target AS (
                SELECT b.id AS bucket_id, o.id AS object_id, m.id AS manifest_id
                FROM access_packages ap
                JOIN application_credentials ac ON ac.id = ap.application_id
                JOIN buckets b ON b.id = ap.bucket_id
                JOIN objects o ON o.id = ap.object_id
                JOIN object_versions v ON v.id = o.current_version_id
                JOIN object_manifests m ON m.object_version_id = v.id
                WHERE ap.id = $1::uuid
                  AND ap.application_id = $2::uuid
                  AND b.name = $3
                  AND o.object_key = $4
                  AND ap.object_manifest_id = m.id
                  AND ap.expires_at > now()
                  AND ap.revoked_at IS NULL
                  AND ac.revoked_at IS NULL
                  AND b.deleted_at IS NULL
                  AND o.deleted_at IS NULL
                  AND o.state = 'AVAILABLE'
            ),
            peer_source AS (
                SELECT id
                FROM peer_fragment_availability
                WHERE id = $11::uuid
                  AND object_manifest_id = (SELECT manifest_id FROM target)
                  AND expires_at > now()
            )
            INSERT INTO fragment_transfer_events (
                source_type, replica_id, bucket_id, object_id, object_manifest_id,
                access_package_id, peer_id, fragment_index, fragment_hash, event_type,
                bytes_transferred, outcome, latency_ms, detail
            )
            SELECT $5, NULL::uuid, bucket_id, object_id, manifest_id,
                $1::uuid,
                CASE WHEN $11::uuid IS NULL THEN NULL ELSE (SELECT id FROM peer_source) END,
                $6, $7, $8, $9, $10, $12, $13
            FROM target
            WHERE $11::uuid IS NULL OR EXISTS (SELECT 1 FROM peer_source)
            RETURNING id::text, source_type, fragment_index, fragment_hash, event_type,
                bytes_transferred, outcome, latency_ms, created_at
            "#,
        )
        .bind(&authorization.package_id)
        .bind(&authorization.application_id)
        .bind(&authorization.bucket_name)
        .bind(&authorization.object_key)
        .bind(&input.source_type)
        .bind(input.fragment_index)
        .bind(&input.fragment_hash)
        .bind(&persisted_event_type)
        .bind(input.bytes_transferred)
        .bind(&persisted_outcome)
        .bind(input.peer_availability_id.as_deref())
        .bind(input.latency_ms)
        .bind(detail)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record SDK fragment event")?;

        let Some(row) = row else {
            bail!("SDK fragment event is not authorized");
        };
        let record = sdk_fragment_event_from_row(row);
        if !hash_matches {
            bail!("fragment hash does not match manifest");
        }
        Ok(record)
    }

    pub async fn record_replica_health(
        &self,
        replica_id: &str,
        input: ReplicaHealthReportInput,
    ) -> anyhow::Result<ReplicaHealthReport> {
        validate_non_negative(input.error_count, "errorCount")?;
        let row = query(
            r#"
            INSERT INTO replica_health_reports (
                replica_id, status, version, storage_available_bytes, error_count, detail
            )
            SELECT id, $2, $3, $4, $5, $6
            FROM replica_credentials
            WHERE id = $1::uuid AND revoked_at IS NULL
            RETURNING replica_id::text, status, version, storage_available_bytes,
                error_count, reported_at
            "#,
        )
        .bind(replica_id)
        .bind(validate_health_status(&input.status)?)
        .bind(input.version)
        .bind(input.storage_available_bytes)
        .bind(input.error_count)
        .bind(input.detail)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record replica health")?;

        let Some(row) = row else {
            bail!("replica not found or revoked");
        };

        Ok(ReplicaHealthReport {
            replica_id: row.get("replica_id"),
            status: row.get("status"),
            version: row.get("version"),
            storage_available_bytes: row.get("storage_available_bytes"),
            error_count: row.get("error_count"),
            reported_at: format_datetime(row.get("reported_at")),
        })
    }

    pub async fn record_replica_metrics(
        &self,
        replica_id: &str,
        input: ReplicaMetricInput,
    ) -> anyhow::Result<ReplicaMetricRecord> {
        validate_non_negative(input.bytes_synced, "bytesSynced")?;
        validate_non_negative(input.bytes_served, "bytesServed")?;
        validate_non_negative(input.fragments_synced, "fragmentsSynced")?;
        validate_non_negative(input.fragments_served, "fragmentsServed")?;
        validate_non_negative(input.sync_failures, "syncFailures")?;
        validate_non_negative(input.auth_failures, "authFailures")?;
        if let Some(value) = input.avg_latency_ms {
            validate_non_negative(value, "avgLatencyMs")?;
        }

        let row = query(
            r#"
            INSERT INTO replica_metric_events (
                replica_id, bytes_synced, bytes_served, fragments_synced,
                fragments_served, sync_failures, auth_failures, avg_latency_ms
            )
            SELECT id, $2, $3, $4, $5, $6, $7, $8
            FROM replica_credentials
            WHERE id = $1::uuid AND revoked_at IS NULL
            RETURNING replica_id::text, bytes_synced, bytes_served, fragments_synced,
                fragments_served, sync_failures, auth_failures, avg_latency_ms, reported_at
            "#,
        )
        .bind(replica_id)
        .bind(input.bytes_synced)
        .bind(input.bytes_served)
        .bind(input.fragments_synced)
        .bind(input.fragments_served)
        .bind(input.sync_failures)
        .bind(input.auth_failures)
        .bind(input.avg_latency_ms)
        .fetch_optional(&self.pool)
        .await
        .context("failed to record replica metrics")?;

        let Some(row) = row else {
            bail!("replica not found or revoked");
        };

        Ok(replica_metric_from_row(row))
    }

    pub async fn replica_traffic_summary(&self) -> anyhow::Result<ReplicaTrafficSummary> {
        let row = query(
            r#"
            SELECT
                (SELECT COUNT(*)::bigint FROM replica_credentials) AS total_replicas,
                (SELECT COUNT(*)::bigint FROM replica_credentials WHERE revoked_at IS NULL) AS active_replicas,
                COALESCE(SUM(bytes_synced), 0)::bigint AS total_bytes_synced,
                COALESCE(SUM(bytes_served), 0)::bigint AS total_bytes_served,
                COALESCE(SUM(fragments_synced), 0)::bigint AS total_fragments_synced,
                COALESCE(SUM(fragments_served), 0)::bigint AS total_fragments_served,
                COALESCE(SUM(sync_failures), 0)::bigint AS sync_failures,
                COALESCE(SUM(auth_failures), 0)::bigint AS auth_failures
            FROM replica_metric_events
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to summarize replica traffic")?;

        Ok(ReplicaTrafficSummary {
            total_replicas: row.get("total_replicas"),
            active_replicas: row.get("active_replicas"),
            total_bytes_synced: row.get("total_bytes_synced"),
            total_bytes_served: row.get("total_bytes_served"),
            total_fragments_synced: row.get("total_fragments_synced"),
            total_fragments_served: row.get("total_fragments_served"),
            sync_failures: row.get("sync_failures"),
            auth_failures: row.get("auth_failures"),
        })
    }

    pub async fn record_replica_sync_transfer(
        &self,
        replica_id: &str,
        bytes_synced: i64,
        fragments_synced: i64,
    ) -> anyhow::Result<()> {
        self.record_replica_metrics(
            replica_id,
            ReplicaMetricInput {
                bytes_synced,
                bytes_served: 0,
                fragments_synced,
                fragments_served: 0,
                sync_failures: 0,
                auth_failures: 0,
                avg_latency_ms: None,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn record_fragment_transfer_event(
        &self,
        source_type: &str,
        replica_id: Option<&str>,
        bucket_name: &str,
        object_key: &str,
        manifest_id: &str,
        fragment_index: i64,
        fragment_hash: &str,
        event_type: &str,
        bytes_transferred: i64,
        outcome: &str,
        detail: serde_json::Value,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        validate_non_negative(bytes_transferred, "bytesTransferred")?;
        let result = query(
            r#"
            WITH target AS (
                SELECT b.id AS bucket_id, o.id AS object_id, m.id AS manifest_id
                FROM buckets b
                JOIN objects o ON o.bucket_id = b.id
                JOIN object_versions v ON v.id = o.current_version_id
                JOIN object_manifests m ON m.object_version_id = v.id
                WHERE b.name = $3
                  AND o.object_key = $4
                  AND m.id = $5::uuid
                  AND b.deleted_at IS NULL
                  AND o.deleted_at IS NULL
            )
            INSERT INTO fragment_transfer_events (
                source_type, replica_id, bucket_id, object_id, object_manifest_id,
                fragment_index, fragment_hash, event_type, bytes_transferred,
                outcome, detail
            )
            SELECT $1, $2::uuid, bucket_id, object_id, manifest_id,
                $6, $7, $8, $9, $10, $11
            FROM target
            "#,
        )
        .bind(source_type)
        .bind(replica_id)
        .bind(bucket_name)
        .bind(object_key)
        .bind(manifest_id)
        .bind(fragment_index)
        .bind(fragment_hash)
        .bind(event_type)
        .bind(bytes_transferred)
        .bind(outcome)
        .bind(detail)
        .execute(&self.pool)
        .await
        .context("failed to record fragment transfer event")?;
        if result.rows_affected() == 0 {
            bail!("fragment transfer target not found");
        }
        Ok(())
    }

    pub async fn list_replica_policy_updates(
        &self,
        replica_id: &str,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<Vec<ReplicaPolicyUpdateRecord>> {
        let rows = query(
            r#"
            SELECT u.id::text, u.update_type, b.name AS bucket_name, o.object_key,
                   u.detail, u.created_at
            FROM replica_policy_updates u
            JOIN replica_credentials r ON r.id = u.replica_id
            LEFT JOIN buckets b ON b.id = u.bucket_id
            LEFT JOIN objects o ON o.id = u.object_id
            WHERE u.replica_id = $1::uuid
              AND (r.revoked_at IS NULL OR u.update_type = 'replica_revoked')
              AND ($2::timestamptz IS NULL OR u.created_at > $2)
            ORDER BY u.created_at ASC
            LIMIT 200
            "#,
        )
        .bind(replica_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .context("failed to list replica policy updates")?;

        Ok(rows
            .into_iter()
            .map(replica_policy_update_from_row)
            .collect())
    }

    pub async fn record_replica_policy_update_for_bucket(
        &self,
        bucket_name: &str,
        object_key: Option<&str>,
        update_type: &str,
        detail: serde_json::Value,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        if let Some(key) = object_key {
            validate_object_key(key)?;
        }
        let result = query(
            r#"
            WITH target AS (
                SELECT b.id AS bucket_id, o.id AS object_id
                FROM buckets b
                LEFT JOIN objects o ON o.bucket_id = b.id AND o.object_key = $2
                WHERE b.name = $1 AND b.deleted_at IS NULL
            )
            INSERT INTO replica_policy_updates (
                replica_id, update_type, bucket_id, object_id, detail
            )
            SELECT r.id, $3, target.bucket_id, target.object_id, $4
            FROM replica_credentials r
            CROSS JOIN target
            WHERE r.revoked_at IS NULL
              AND r.allowed_buckets ? $1
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .bind(update_type)
        .bind(detail)
        .execute(&self.pool)
        .await
        .context("failed to record replica policy update")?;
        if result.rows_affected() == 0 {
            // No active replica needed this update; that is not an error.
        }
        Ok(())
    }

    pub async fn create_access_package(
        &self,
        application_id: &str,
        bucket_name: &str,
        object_key: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<AccessPackageRecord> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;

        let package_token = secure_url_token("pm_ap_", 32);
        let package_token_hash = hash_bearer_token(&package_token);
        let row = query(
            r#"
            WITH target AS (
                SELECT b.id AS bucket_id, o.id AS object_id, m.id AS manifest_id
                FROM buckets b
                JOIN objects o ON o.bucket_id = b.id
                JOIN object_versions v ON v.id = o.current_version_id
                JOIN object_manifests m ON m.object_version_id = v.id
                WHERE b.name = $3 AND o.object_key = $4
                  AND b.deleted_at IS NULL AND o.deleted_at IS NULL
                  AND o.state = 'AVAILABLE'
            )
            INSERT INTO access_packages (
                package_token_hash, application_id, bucket_id, object_id, object_manifest_id, expires_at
            )
            SELECT $1, $2::uuid, bucket_id, object_id, manifest_id, $5
            FROM target
            RETURNING id::text, object_manifest_id::text, expires_at, created_at
            "#,
        )
        .bind(package_token_hash)
        .bind(application_id)
        .bind(bucket_name)
        .bind(object_key)
        .bind(expires_at)
        .fetch_optional(&self.pool)
        .await
        .context("failed to create access package")?;

        let Some(row) = row else {
            bail!("object not found");
        };

        Ok(AccessPackageRecord {
            id: row.get("id"),
            package_token,
            application_id: application_id.to_owned(),
            bucket_name: bucket_name.to_owned(),
            object_key: object_key.to_owned(),
            manifest_id: row.get("object_manifest_id"),
            expires_at: format_datetime(row.get("expires_at")),
            created_at: format_datetime(row.get("created_at")),
            signature_algorithm: None,
            signature: None,
        })
    }

    pub async fn store_object_manifest_signature(
        &self,
        manifest_id: &str,
        algorithm: &str,
        signature: &str,
    ) -> anyhow::Result<()> {
        query(
            r#"
            UPDATE object_manifests
            SET signature_algorithm = $2, signature = $3
            WHERE id = $1::uuid
            "#,
        )
        .bind(manifest_id)
        .bind(algorithm)
        .bind(signature)
        .execute(&self.pool)
        .await
        .context("failed to store object manifest signature")?;
        Ok(())
    }

    pub async fn store_access_package_signature(
        &self,
        package_id: &str,
        algorithm: &str,
        signature: &str,
    ) -> anyhow::Result<()> {
        query(
            r#"
            UPDATE access_packages
            SET signature_algorithm = $2, signature = $3
            WHERE id = $1::uuid
            "#,
        )
        .bind(package_id)
        .bind(algorithm)
        .bind(signature)
        .execute(&self.pool)
        .await
        .context("failed to store access package signature")?;
        Ok(())
    }

    pub async fn authorize_access_package(
        &self,
        package_id: &str,
        package_token_hash: &str,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<AccessPackageAuthorization>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = query(
            r#"
            SELECT
                ap.id::text AS package_id,
                ap.application_id::text AS application_id,
                b.name AS bucket_name,
                o.object_key,
                ap.object_manifest_id::text AS manifest_id
            FROM access_packages ap
            JOIN application_credentials ac ON ac.id = ap.application_id
            JOIN buckets b ON b.id = ap.bucket_id
            JOIN objects o ON o.id = ap.object_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN object_manifests m ON m.object_version_id = v.id
            WHERE ap.id = $1::uuid
              AND ap.package_token_hash = $2
              AND b.name = $3
              AND o.object_key = $4
              AND ap.expires_at > now()
              AND ap.revoked_at IS NULL
              AND ac.revoked_at IS NULL
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND ap.object_manifest_id = m.id
            "#,
        )
        .bind(package_id)
        .bind(package_token_hash)
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to authorize access package")?;

        Ok(row.map(|row| AccessPackageAuthorization {
            package_id: row.get("package_id"),
            application_id: row.get("application_id"),
            bucket_name: row.get("bucket_name"),
            object_key: row.get("object_key"),
            manifest_id: row.get("manifest_id"),
        }))
    }

    pub async fn revoke_access_package(&self, package_id: &str) -> anyhow::Result<()> {
        let result = query(
            "UPDATE access_packages SET revoked_at = now() WHERE id = $1::uuid AND revoked_at IS NULL",
        )
        .bind(package_id)
        .execute(&self.pool)
        .await
        .context("failed to revoke access package")?;
        if result.rows_affected() == 0 {
            bail!("access package not found or already revoked: {package_id}");
        }
        Ok(())
    }

    pub async fn record_audit_event(
        &self,
        event: &str,
        principal: Option<&str>,
        outcome: &str,
        detail: &str,
    ) -> anyhow::Result<AuditEventRecord> {
        let metadata = serde_json::json!({
            "principal": principal,
            "outcome": outcome,
            "detail": detail
        });
        let row = query(
            r#"
            INSERT INTO audit_events (event_type, metadata)
            VALUES ($1, $2)
            RETURNING id::text, event_type, metadata, created_at
            "#,
        )
        .bind(event)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await
        .context("failed to record audit event")?;

        Ok(audit_event_from_row(row))
    }

    pub async fn list_audit_events_filtered(
        &self,
        filter: AuditEventFilter,
    ) -> anyhow::Result<Vec<AuditEventRecord>> {
        let rows = query(
            r#"
            SELECT id::text, event_type, metadata, created_at
            FROM audit_events
            WHERE ($1::text IS NULL OR event_type = $1)
              AND ($2::text IS NULL OR metadata->>'principal' = $2)
              AND ($3::text IS NULL OR metadata->>'outcome' = $3)
              AND ($4::timestamptz IS NULL OR created_at >= $4)
              AND ($5::timestamptz IS NULL OR created_at <= $5)
            ORDER BY created_at DESC
            LIMIT $6
            "#,
        )
        .bind(filter.event)
        .bind(filter.principal)
        .bind(filter.outcome)
        .bind(filter.since)
        .bind(filter.until)
        .bind(filter.limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .context("failed to list filtered audit events")?;

        Ok(rows.into_iter().map(audit_event_from_row).collect())
    }

    pub async fn count_recent_login_failures(
        &self,
        principal: &str,
        window_seconds: i64,
    ) -> anyhow::Result<i64> {
        let row = query(
            r#"
            SELECT COUNT(*)::bigint AS failed_attempts
            FROM audit_events
            WHERE event_type = 'login_failed'
              AND metadata->>'principal' = $1
              AND created_at > now() - make_interval(secs => $2::double precision)
            "#,
        )
        .bind(principal)
        .bind(window_seconds.max(1) as f64)
        .fetch_one(&self.pool)
        .await
        .context("failed to count recent login failures")?;

        Ok(row.get("failed_attempts"))
    }

    pub async fn record_origin_transfer(
        &self,
        application_id: Option<&str>,
        bucket_name: &str,
        object_key: &str,
        bytes_served: i64,
        range: Option<(u64, u64)>,
        status_code: u16,
    ) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let (range_start, range_end): (Option<i64>, Option<i64>) = match range {
            Some((start, end)) => (
                Some(i64::try_from(start).context("range start is too large")?),
                Some(i64::try_from(end).context("range end is too large")?),
            ),
            None => (None, None),
        };

        let result = query(
            r#"
            WITH target AS (
                SELECT b.id AS bucket_id, o.id AS object_id
                FROM buckets b
                JOIN objects o ON o.bucket_id = b.id
                WHERE b.name = $2 AND o.object_key = $3
                  AND b.deleted_at IS NULL AND o.deleted_at IS NULL
            )
            INSERT INTO origin_transfer_events (
                application_id, bucket_id, object_id, bytes_served,
                range_start, range_end, status_code
            )
            SELECT $1::uuid, bucket_id, object_id, $4, $5, $6, $7
            FROM target
            "#,
        )
        .bind(application_id)
        .bind(bucket_name)
        .bind(object_key)
        .bind(bytes_served)
        .bind(range_start)
        .bind(range_end)
        .bind(i32::from(status_code))
        .execute(&self.pool)
        .await
        .context("failed to record Origin transfer")?;
        if result.rows_affected() == 0 {
            bail!("object not found");
        }
        Ok(())
    }

    pub async fn origin_traffic_summary(&self) -> anyhow::Result<OriginTrafficSummary> {
        let row = query(
            r#"
            SELECT
                COUNT(*)::bigint AS total_requests,
                COALESCE(SUM(CASE WHEN range_start IS NULL THEN 1 ELSE 0 END), 0)::bigint AS full_object_requests,
                COALESCE(SUM(CASE WHEN range_start IS NOT NULL THEN 1 ELSE 0 END), 0)::bigint AS range_requests,
                COALESCE(SUM(bytes_served), 0)::bigint AS total_bytes_served,
                (
                    SELECT COALESCE(SUM(CASE WHEN event_type = 'FALLBACK_DECISION' THEN 1 ELSE 0 END), 0)::bigint
                    FROM fragment_transfer_events
                ) AS fallback_events,
                (
                    SELECT COALESCE(SUM(CASE WHEN event_type = 'HASH_MISMATCH' OR outcome = 'REJECTED' THEN 1 ELSE 0 END), 0)::bigint
                    FROM fragment_transfer_events
                ) AS integrity_failures,
                (
                    SELECT COALESCE(SUM(CASE WHEN source_type IN ('REPLICA_EDGE', 'PEER') AND outcome = 'SUCCESS' AND event_type <> 'FRAGMENT_SYNCED' THEN bytes_transferred ELSE 0 END), 0)::bigint
                    FROM fragment_transfer_events
                ) AS origin_offload_bytes
            FROM origin_transfer_events
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to summarize Origin traffic")?;

        Ok(OriginTrafficSummary {
            total_requests: row.get("total_requests"),
            full_object_requests: row.get("full_object_requests"),
            range_requests: row.get("range_requests"),
            total_bytes_served: row.get("total_bytes_served"),
            fallback_events: row.get("fallback_events"),
            integrity_failures: row.get("integrity_failures"),
            origin_offload_bytes: row.get("origin_offload_bytes"),
        })
    }

    pub async fn bucket_traffic_metrics(&self) -> anyhow::Result<Vec<BucketTrafficMetric>> {
        let rows = query(
            r#"
            WITH origin AS (
                SELECT bucket_id, SUM(bytes_served)::bigint AS bytes_served,
                       COUNT(*)::bigint AS requests
                FROM origin_transfer_events
                GROUP BY bucket_id
            ),
            fragments AS (
                SELECT bucket_id,
                       SUM(CASE WHEN source_type = 'REPLICA_EDGE' THEN bytes_transferred ELSE 0 END)::bigint AS replica_bytes_synced,
                       SUM(CASE WHEN source_type = 'PEER' AND outcome = 'SUCCESS' THEN bytes_transferred ELSE 0 END)::bigint AS peer_bytes_served,
                       SUM(CASE WHEN event_type = 'FALLBACK_DECISION' THEN 1 ELSE 0 END)::bigint AS fallback_events,
                       SUM(CASE WHEN event_type = 'HASH_MISMATCH' OR outcome = 'REJECTED' THEN 1 ELSE 0 END)::bigint AS integrity_failures,
                       SUM(CASE WHEN source_type IN ('REPLICA_EDGE', 'PEER') AND outcome = 'SUCCESS' AND event_type <> 'FRAGMENT_SYNCED' THEN bytes_transferred ELSE 0 END)::bigint AS origin_offload_bytes,
                       SUM(CASE WHEN event_type IN ('FRAGMENT_ATTEMPTED', 'FRAGMENT_SYNCED', 'FRAGMENT_SERVED', 'FALLBACK_DECISION', 'HASH_MISMATCH') THEN 1 ELSE 0 END)::bigint AS source_attempts,
                       AVG(CASE WHEN source_type IN ('REPLICA_EDGE', 'PEER') THEN latency_ms ELSE NULL END)::float8 AS avg_auxiliary_latency_ms,
                       COUNT(*)::bigint AS fragment_events
                FROM fragment_transfer_events
                GROUP BY bucket_id
            )
            SELECT
                b.name AS bucket_name,
                COALESCE(origin.bytes_served, 0)::bigint AS origin_bytes_served,
                COALESCE(origin.requests, 0)::bigint AS origin_requests,
                COALESCE(fragments.replica_bytes_synced, 0)::bigint AS replica_bytes_synced,
                COALESCE(fragments.peer_bytes_served, 0)::bigint AS peer_bytes_served,
                COALESCE(fragments.fragment_events, 0)::bigint AS fragment_events,
                COALESCE(fragments.fallback_events, 0)::bigint AS fallback_events,
                COALESCE(fragments.integrity_failures, 0)::bigint AS integrity_failures,
                COALESCE(fragments.origin_offload_bytes, 0)::bigint AS origin_offload_bytes,
                COALESCE(fragments.source_attempts, 0)::bigint AS source_attempts,
                CASE
                    WHEN COALESCE(fragments.source_attempts, 0) = 0 THEN 0::float8
                    ELSE fragments.fallback_events::float8 / fragments.source_attempts::float8
                END AS fallback_rate,
                CASE
                    WHEN COALESCE(fragments.source_attempts, 0) = 0 THEN 0::float8
                    ELSE fragments.integrity_failures::float8 / fragments.source_attempts::float8
                END AS integrity_failure_rate,
                fragments.avg_auxiliary_latency_ms
            FROM buckets b
            LEFT JOIN origin ON origin.bucket_id = b.id
            LEFT JOIN fragments ON fragments.bucket_id = b.id
            WHERE b.deleted_at IS NULL
            ORDER BY b.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to load bucket traffic metrics")?;

        Ok(rows
            .into_iter()
            .map(|row| BucketTrafficMetric {
                bucket: row.get("bucket_name"),
                origin_bytes_served: row.get("origin_bytes_served"),
                origin_requests: row.get("origin_requests"),
                replica_bytes_synced: row.get("replica_bytes_synced"),
                peer_bytes_served: row.get("peer_bytes_served"),
                fragment_events: row.get("fragment_events"),
                fallback_events: row.get("fallback_events"),
                integrity_failures: row.get("integrity_failures"),
                origin_offload_bytes: row.get("origin_offload_bytes"),
                source_attempts: row.get("source_attempts"),
                fallback_rate: row.get("fallback_rate"),
                integrity_failure_rate: row.get("integrity_failure_rate"),
                avg_auxiliary_latency_ms: row.get("avg_auxiliary_latency_ms"),
            })
            .collect())
    }

    pub async fn object_traffic_metrics(&self) -> anyhow::Result<Vec<ObjectTrafficMetric>> {
        let rows = query(
            r#"
            WITH origin AS (
                SELECT object_id, SUM(bytes_served)::bigint AS bytes_served,
                       COUNT(*)::bigint AS requests
                FROM origin_transfer_events
                GROUP BY object_id
            ),
            fragments AS (
                SELECT object_id,
                       SUM(CASE WHEN source_type = 'REPLICA_EDGE' THEN bytes_transferred ELSE 0 END)::bigint AS replica_bytes_synced,
                       SUM(CASE WHEN source_type = 'PEER' AND outcome = 'SUCCESS' THEN bytes_transferred ELSE 0 END)::bigint AS peer_bytes_served,
                       SUM(CASE WHEN event_type = 'FALLBACK_DECISION' THEN 1 ELSE 0 END)::bigint AS fallback_events,
                       SUM(CASE WHEN event_type = 'HASH_MISMATCH' OR outcome = 'REJECTED' THEN 1 ELSE 0 END)::bigint AS integrity_failures,
                       SUM(CASE WHEN source_type IN ('REPLICA_EDGE', 'PEER') AND outcome = 'SUCCESS' AND event_type <> 'FRAGMENT_SYNCED' THEN bytes_transferred ELSE 0 END)::bigint AS origin_offload_bytes,
                       SUM(CASE WHEN event_type IN ('FRAGMENT_ATTEMPTED', 'FRAGMENT_SYNCED', 'FRAGMENT_SERVED', 'FALLBACK_DECISION', 'HASH_MISMATCH') THEN 1 ELSE 0 END)::bigint AS source_attempts,
                       AVG(CASE WHEN source_type IN ('REPLICA_EDGE', 'PEER') THEN latency_ms ELSE NULL END)::float8 AS avg_auxiliary_latency_ms,
                       COUNT(*)::bigint AS fragment_events
                FROM fragment_transfer_events
                GROUP BY object_id
            )
            SELECT
                b.name AS bucket_name,
                o.object_key,
                COALESCE(origin.bytes_served, 0)::bigint AS origin_bytes_served,
                COALESCE(origin.requests, 0)::bigint AS origin_requests,
                COALESCE(fragments.replica_bytes_synced, 0)::bigint AS replica_bytes_synced,
                COALESCE(fragments.peer_bytes_served, 0)::bigint AS peer_bytes_served,
                COALESCE(fragments.fragment_events, 0)::bigint AS fragment_events,
                COALESCE(fragments.fallback_events, 0)::bigint AS fallback_events,
                COALESCE(fragments.integrity_failures, 0)::bigint AS integrity_failures,
                COALESCE(fragments.origin_offload_bytes, 0)::bigint AS origin_offload_bytes,
                COALESCE(fragments.source_attempts, 0)::bigint AS source_attempts,
                CASE
                    WHEN COALESCE(fragments.source_attempts, 0) = 0 THEN 0::float8
                    ELSE fragments.fallback_events::float8 / fragments.source_attempts::float8
                END AS fallback_rate,
                CASE
                    WHEN COALESCE(fragments.source_attempts, 0) = 0 THEN 0::float8
                    ELSE fragments.integrity_failures::float8 / fragments.source_attempts::float8
                END AS integrity_failure_rate,
                fragments.avg_auxiliary_latency_ms
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            LEFT JOIN origin ON origin.object_id = o.id
            LEFT JOIN fragments ON fragments.object_id = o.id
            WHERE b.deleted_at IS NULL
              AND o.deleted_at IS NULL
            ORDER BY b.name ASC, o.object_key ASC
            LIMIT 500
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to load object traffic metrics")?;

        Ok(rows
            .into_iter()
            .map(|row| ObjectTrafficMetric {
                bucket: row.get("bucket_name"),
                key: row.get("object_key"),
                origin_bytes_served: row.get("origin_bytes_served"),
                origin_requests: row.get("origin_requests"),
                replica_bytes_synced: row.get("replica_bytes_synced"),
                peer_bytes_served: row.get("peer_bytes_served"),
                fragment_events: row.get("fragment_events"),
                fallback_events: row.get("fallback_events"),
                integrity_failures: row.get("integrity_failures"),
                origin_offload_bytes: row.get("origin_offload_bytes"),
                source_attempts: row.get("source_attempts"),
                fallback_rate: row.get("fallback_rate"),
                integrity_failure_rate: row.get("integrity_failure_rate"),
                avg_auxiliary_latency_ms: row.get("avg_auxiliary_latency_ms"),
            })
            .collect())
    }

    pub async fn replica_detail_metrics(
        &self,
        replica_id: &str,
    ) -> anyhow::Result<Option<ReplicaDetailMetric>> {
        let row = query(
            r#"
            WITH metrics AS (
                SELECT replica_id,
                       SUM(bytes_synced)::bigint AS bytes_synced,
                       SUM(bytes_served)::bigint AS bytes_served,
                       SUM(fragments_synced)::bigint AS fragments_synced,
                       SUM(fragments_served)::bigint AS fragments_served,
                       SUM(sync_failures)::bigint AS sync_failures,
                       SUM(auth_failures)::bigint AS auth_failures
                FROM replica_metric_events
                GROUP BY replica_id
            ),
            fragments AS (
                SELECT replica_id, COUNT(*)::bigint AS fragment_events
                FROM fragment_transfer_events
                GROUP BY replica_id
            )
            SELECT
                r.id::text AS replica_id,
                r.name AS replica_name,
                COALESCE(metrics.bytes_synced, 0)::bigint AS bytes_synced,
                COALESCE(metrics.bytes_served, 0)::bigint AS bytes_served,
                COALESCE(metrics.fragments_synced, 0)::bigint AS fragments_synced,
                COALESCE(metrics.fragments_served, 0)::bigint AS fragments_served,
                COALESCE(metrics.sync_failures, 0)::bigint AS sync_failures,
                COALESCE(metrics.auth_failures, 0)::bigint AS auth_failures,
                COALESCE(fragments.fragment_events, 0)::bigint AS fragment_events
            FROM replica_credentials r
            LEFT JOIN metrics ON metrics.replica_id = r.id
            LEFT JOIN fragments ON fragments.replica_id = r.id
            WHERE r.id = $1::uuid
            "#,
        )
        .bind(replica_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load replica detail metrics")?;

        Ok(row.map(|row| ReplicaDetailMetric {
            replica_id: row.get("replica_id"),
            replica_name: row.get("replica_name"),
            bytes_synced: row.get("bytes_synced"),
            bytes_served: row.get("bytes_served"),
            fragments_synced: row.get("fragments_synced"),
            fragments_served: row.get("fragments_served"),
            sync_failures: row.get("sync_failures"),
            auth_failures: row.get("auth_failures"),
            fragment_events: row.get("fragment_events"),
        }))
    }
}

async fn bucket_id_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    bucket_name: &str,
) -> anyhow::Result<String> {
    let row =
        query("SELECT id::text FROM buckets WHERE name = $1 AND deleted_at IS NULL FOR UPDATE")
            .bind(bucket_name)
            .fetch_optional(&mut **tx)
            .await
            .context("failed to load bucket")?;
    row.map(|row| row.get("id"))
        .ok_or_else(|| anyhow::anyhow!("bucket not found: {bucket_name}"))
}

async fn insert_object_shell_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    bucket_id: &str,
    object_key: &str,
) -> anyhow::Result<String> {
    let existing = query(
        r#"
        SELECT id::text, deleted_at IS NULL AS active
        FROM objects
        WHERE bucket_id = $1::uuid AND object_key = $2
        FOR UPDATE
        "#,
    )
    .bind(bucket_id)
    .bind(object_key)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to load existing object")?;

    if let Some(row) = existing {
        let object_id = row.get::<String, _>("id");
        if row.get::<bool, _>("active") {
            tracing::info!(
                object_id = %object_id,
                object_key = %object_key,
                "object_put_existing_active_found"
            );
            bail!("active object already exists in bucket");
        }

        tracing::info!(
            object_id = %object_id,
            object_key = %object_key,
            "object_put_deleted_key_reused"
        );
        query(
            r#"
            UPDATE objects
            SET state = 'AVAILABLE', deleted_at = NULL, updated_at = now()
            WHERE id = $1::uuid
            "#,
        )
        .bind(&object_id)
        .execute(&mut **tx)
        .await
        .context("failed to reactivate deleted object")?;
        return Ok(object_id);
    }

    let row = query(
        r#"
        INSERT INTO objects (bucket_id, object_key)
        VALUES ($1::uuid, $2)
        RETURNING id::text
        "#,
    )
    .bind(bucket_id)
    .bind(object_key)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_unique_violation("object already exists in bucket"))?;
    Ok(row.get("id"))
}

async fn upsert_object_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    bucket_id: &str,
    object_key: &str,
) -> anyhow::Result<String> {
    let row = query(
        r#"
        INSERT INTO objects (bucket_id, object_key)
        VALUES ($1::uuid, $2)
        ON CONFLICT (bucket_id, object_key) DO UPDATE SET
            updated_at = now(),
            deleted_at = NULL
        RETURNING id::text
        "#,
    )
    .bind(bucket_id)
    .bind(object_key)
    .fetch_one(&mut **tx)
    .await
    .context("failed to upsert object")?;
    Ok(row.get("id"))
}

async fn record_audit_event_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    event: &str,
    actor_user_id: Option<&str>,
    ip_address: Option<IpAddr>,
    user_agent: Option<&str>,
    metadata: serde_json::Value,
) -> anyhow::Result<()> {
    query(
        r#"
        INSERT INTO audit_events (
            event_type, actor_user_id, ip_address, user_agent, metadata
        )
        VALUES ($1, $2::uuid, $3::inet, $4, $5)
        "#,
    )
    .bind(event)
    .bind(actor_user_id)
    .bind(ip_address.map(|value| value.to_string()))
    .bind(user_agent)
    .bind(metadata)
    .execute(&mut **tx)
    .await
    .context("failed to record audit event")?;
    Ok(())
}

fn object_summary_from_row(row: PgRow) -> ObjectSummary {
    ObjectSummary {
        key: row.get("object_key"),
        size_bytes: row.get("size_bytes"),
        content_type: row
            .get::<Option<String>, _>("content_type")
            .unwrap_or_else(|| "application/octet-stream".to_owned()),
        sha256: row.get("object_hash"),
        version_id: row.try_get("s3_version_id").ok(),
        is_delete_marker: row.try_get("is_delete_marker").unwrap_or(false),
        created_at: format_datetime(row.get("created_at")),
        updated_at: format_datetime(row.get("updated_at")),
        state: row.get("state"),
        created_by: row.try_get("created_by").ok().flatten(),
    }
}

fn object_record_from_row(row: PgRow) -> ObjectRecord {
    ObjectRecord {
        key: row.get("object_key"),
        size_bytes: row.get("size_bytes"),
        content_type: row
            .get::<Option<String>, _>("content_type")
            .unwrap_or_else(|| "application/octet-stream".to_owned()),
        sha256: row.get("object_hash"),
        storage_path: row.get("storage_path"),
        version_id: row.get("s3_version_id"),
        is_delete_marker: row.get("is_delete_marker"),
        checksum_sha256: row.get("checksum_sha256"),
        checksum_crc32: row.get("checksum_crc32"),
        encryption_algorithm: row.get("encryption_algorithm"),
        encryption_key_id: row.get("encryption_key_id"),
        encryption_nonce: row.get("encryption_nonce"),
        object_lock_mode: row.get("object_lock_mode"),
        retain_until: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("retain_until")
            .map(format_datetime),
        legal_hold: row.get("legal_hold"),
        created_at: format_datetime(row.get("created_at")),
        state: row.get("state"),
        user_metadata: row.try_get("user_metadata").ok().flatten(),
        created_by: row.try_get("created_by").ok().flatten(),
    }
}

fn mcp_settings_from_row(row: PgRow) -> McpSettings {
    McpSettings {
        enabled: row.get("enabled"),
        endpoint_path: row.get("endpoint_path"),
        bind_host: row.get("bind_host"),
        require_auth: row.get("require_auth"),
        read_tools_enabled: row.get("read_tools_enabled"),
        write_tools_enabled: row.get("write_tools_enabled"),
        admin_tools_enabled: row.get("admin_tools_enabled"),
        expose_resources: row.get("expose_resources"),
        expose_prompts: row.get("expose_prompts"),
        allow_localhost_only: row.get("allow_localhost_only"),
        created_at: format_datetime(row.get("created_at")),
        updated_at: format_datetime(row.get("updated_at")),
    }
}

fn mcp_access_token_from_row(row: PgRow) -> McpAccessTokenSummary {
    McpAccessTokenSummary {
        id: row.get("id"),
        name: row.get("name"),
        token_prefix: row.get("token_prefix"),
        active: row.get::<bool, _>("is_active")
            && row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
                .is_none(),
        created_at: format_datetime(row.get("created_at")),
        revoked_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
            .map(format_datetime),
        last_used_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_used_at")
            .map(format_datetime),
        scopes: row.get("scopes"),
    }
}

fn mcp_activity_from_row(row: PgRow) -> McpActivityRecord {
    McpActivityRecord {
        id: row.get("id"),
        token_id: row.get("token_id"),
        method: row.get("method"),
        target: row.get("target"),
        outcome: row.get("outcome"),
        created_at: format_datetime(row.get("created_at")),
    }
}

fn multipart_upload_from_row(row: PgRow) -> MultipartUploadRecord {
    MultipartUploadRecord {
        upload_id: row.get("upload_id"),
        bucket_name: row.get("bucket_name"),
        object_key: row.get("object_key"),
        content_type: row.get("content_type"),
        initiated_at: format_datetime(row.get("initiated_at")),
        user_metadata: row.try_get("user_metadata").ok().flatten(),
    }
}

fn multipart_part_from_row(row: PgRow) -> MultipartPartRecord {
    MultipartPartRecord {
        part_number: row.get("part_number"),
        etag: row.get("etag"),
        size_bytes: row.get("size_bytes"),
        storage_path: row.get("storage_path"),
        uploaded_at: format_datetime(row.get("uploaded_at")),
    }
}

fn bucket_policy_from_row(row: PgRow) -> BucketPolicy {
    BucketPolicy {
        bucket_name: row.get("name"),
        access_package_ttl_seconds: row.get("access_package_ttl_seconds"),
        fragment_size_bytes: row.get("fragment_size_bytes"),
        allow_replica_edge: row.get("allow_replica_edge"),
        allow_peer_sharing: row.get("allow_peer_sharing"),
        source_selection_strategy: row.get("source_selection_strategy"),
        fragment_priority_strategy: row.get("fragment_priority_strategy"),
        failure_threshold: row.get("failure_threshold"),
        fallback_mode: row.get("fallback_mode"),
        s3_list_default_max_keys: row.get("s3_list_default_max_keys"),
        s3_list_max_keys_limit: row.get("s3_list_max_keys_limit"),
        s3_list_allow_delimiter: row.get("s3_list_allow_delimiter"),
        s3_versioning_enabled: row.get("s3_versioning_enabled"),
        s3_object_tagging_enabled: row.get("s3_object_tagging_enabled"),
        s3_checksum_algorithm: row.get("s3_checksum_algorithm"),
        s3_multipart_abort_days: row.get("s3_multipart_abort_days"),
        s3_default_encryption_algorithm: row.get("s3_default_encryption_algorithm"),
        s3_default_encryption_key_id: row.get("s3_default_encryption_key_id"),
        s3_object_lock_enabled: row.get("s3_object_lock_enabled"),
        s3_object_lock_default_mode: row.get("s3_object_lock_default_mode"),
        s3_object_lock_default_retain_days: row.get("s3_object_lock_default_retain_days"),
        s3_lifecycle_rules: row.get("s3_lifecycle_rules"),
        s3_resource_policy: row.get("s3_resource_policy"),
        s3_event_notifications: row.get("s3_event_notifications"),
        updated_at: format_datetime(row.get("updated_at")),
    }
}

fn bucket_policy_defaults_from_row(row: PgRow) -> BucketPolicyDefaults {
    BucketPolicyDefaults {
        access_package_ttl_seconds: row.get("access_package_ttl_seconds"),
        fragment_size_bytes: row.get("fragment_size_bytes"),
        allow_replica_edge: row.get("allow_replica_edge"),
        allow_peer_sharing: row.get("allow_peer_sharing"),
        source_selection_strategy: row.get("source_selection_strategy"),
        fragment_priority_strategy: row.get("fragment_priority_strategy"),
        failure_threshold: row.get("failure_threshold"),
        fallback_mode: row.get("fallback_mode"),
        updated_at: format_datetime(row.get("updated_at")),
    }
}

fn application_summary_from_row(row: PgRow) -> anyhow::Result<ApplicationCredentialSummary> {
    Ok(ApplicationCredentialSummary {
        id: row.get("id"),
        name: row.get("name"),
        scopes: parse_string_vec(row.get("scopes"))?,
        created_at: format_datetime(row.get("created_at")),
        revoked: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
            .is_some(),
    })
}

fn s3_access_key_summary_from_row(row: PgRow) -> S3AccessKeySummary {
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.get("revoked_at");
    let last_used_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_used_at");
    S3AccessKeySummary {
        id: row.get("id"),
        name: row.get("name"),
        access_key_id: row.get("access_key_id"),
        user_id: row.get("user_id"),
        is_active: row.get("is_active"),
        created_at: format_datetime(row.get("created_at")),
        revoked_at: revoked_at.map(format_datetime),
        last_used_at: last_used_at.map(format_datetime),
    }
}

fn total_pages(total: i64, page_size: u32) -> u32 {
    if total <= 0 {
        return 1;
    }
    let page_size = i64::from(page_size.max(1));
    ((total + page_size - 1) / page_size) as u32
}

fn normalize_optional_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn replica_summary_from_row(row: PgRow) -> anyhow::Result<ReplicaSummary> {
    let last_seen_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_seen_at");
    let health_reported_at: Option<chrono::DateTime<chrono::Utc>> = row.get("health_reported_at");
    Ok(ReplicaSummary {
        id: row.get("id"),
        name: row.get("name"),
        allowed_buckets: parse_string_vec(row.get("allowed_buckets"))?,
        created_at: format_datetime(row.get("created_at")),
        revoked: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
            .is_some(),
        available_objects: row.get("available_objects"),
        last_seen_at: last_seen_at.map(format_datetime),
        health_status: row.get("health_status"),
        health_reported_at: health_reported_at.map(format_datetime),
    })
}

fn availability_record_from_row(row: PgRow) -> anyhow::Result<ReplicaAvailabilityRecord> {
    Ok(ReplicaAvailabilityRecord {
        replica_id: row.get("replica_id"),
        replica_name: row.get("replica_name"),
        bucket: row.get("bucket_name"),
        key: row.get("object_key"),
        endpoint: row.get("endpoint"),
        available_fragments: parse_i64_vec(row.get("available_fragments"))?,
        last_seen_at: format_datetime(row.get("last_seen_at")),
    })
}

fn peer_availability_record_from_row(row: PgRow) -> anyhow::Result<PeerAvailabilityRecord> {
    Ok(PeerAvailabilityRecord {
        id: row.get("id"),
        peer_id: row.get("peer_id"),
        bucket: row.get("bucket_name"),
        key: row.get("object_key"),
        endpoint: row.get("endpoint"),
        available_fragments: parse_i64_vec(row.get("available_fragments"))?,
        expires_at: format_datetime(row.get("expires_at")),
        last_seen_at: format_datetime(row.get("last_seen_at")),
    })
}

fn sdk_fragment_event_from_row(row: PgRow) -> SdkFragmentEventRecord {
    SdkFragmentEventRecord {
        id: row.get("id"),
        source_type: row.get("source_type"),
        fragment_index: row.get("fragment_index"),
        fragment_hash: row.get("fragment_hash"),
        event_type: row.get("event_type"),
        bytes_transferred: row.get("bytes_transferred"),
        outcome: row.get("outcome"),
        latency_ms: row.get("latency_ms"),
        created_at: format_datetime(row.get("created_at")),
    }
}

fn audit_event_from_row(row: PgRow) -> AuditEventRecord {
    let metadata = row
        .get::<Option<serde_json::Value>, _>("metadata")
        .unwrap_or_else(|| serde_json::json!({}));
    AuditEventRecord {
        id: row.get("id"),
        event: row.get("event_type"),
        principal: metadata
            .get("principal")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        outcome: metadata
            .get("outcome")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned(),
        detail: metadata
            .get("detail")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned(),
        created_at: format_datetime(row.get("created_at")),
    }
}

fn replica_metric_from_row(row: PgRow) -> ReplicaMetricRecord {
    ReplicaMetricRecord {
        replica_id: row.get("replica_id"),
        bytes_synced: row.get("bytes_synced"),
        bytes_served: row.get("bytes_served"),
        fragments_synced: row.get("fragments_synced"),
        fragments_served: row.get("fragments_served"),
        sync_failures: row.get("sync_failures"),
        auth_failures: row.get("auth_failures"),
        avg_latency_ms: row.get("avg_latency_ms"),
        reported_at: format_datetime(row.get("reported_at")),
    }
}

fn replica_policy_update_from_row(row: PgRow) -> ReplicaPolicyUpdateRecord {
    ReplicaPolicyUpdateRecord {
        id: row.get("id"),
        update_type: row.get("update_type"),
        bucket: row.get("bucket_name"),
        object_key: row.get("object_key"),
        detail: row
            .get::<Option<serde_json::Value>, _>("detail")
            .unwrap_or_else(|| serde_json::json!({})),
        created_at: format_datetime(row.get("created_at")),
    }
}

fn parse_string_vec(value: serde_json::Value) -> anyhow::Result<Vec<String>> {
    serde_json::from_value(value).context("failed to parse string list")
}

fn parse_i64_vec(value: serde_json::Value) -> anyhow::Result<Vec<i64>> {
    serde_json::from_value(value).context("failed to parse integer list")
}

fn parse_fragment_id(value: &str) -> anyhow::Result<(String, i64, String)> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        bail!("fragmentId must use manifestId:index:sha256 format");
    }
    let index = parts[1]
        .parse::<i64>()
        .context("fragment index is invalid")?;
    if index < 0 {
        bail!("fragment index must be non-negative");
    }
    if parts[2].len() != 64 || !parts[2].chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("fragment hash must be a SHA-256 hex digest");
    }
    Ok((parts[0].to_owned(), index, parts[2].to_owned()))
}

fn format_datetime(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339()
}

fn validate_bucket_policy(update: &BucketPolicyUpdate) -> anyhow::Result<()> {
    validate_hybrid_policy(
        update.access_package_ttl_seconds,
        update.fragment_size_bytes,
        &update.source_selection_strategy,
        &update.fragment_priority_strategy,
        update.failure_threshold,
        &update.fallback_mode,
    )?;
    if !(1..=10_000).contains(&update.s3_list_default_max_keys) {
        bail!("s3ListDefaultMaxKeys must be between 1 and 10000");
    }
    if !(1..=100_000).contains(&update.s3_list_max_keys_limit) {
        bail!("s3ListMaxKeysLimit must be between 1 and 100000");
    }
    if update.s3_list_default_max_keys > update.s3_list_max_keys_limit {
        bail!("s3ListDefaultMaxKeys cannot be greater than s3ListMaxKeysLimit");
    }
    validate_policy_enum(
        "s3ChecksumAlgorithm",
        &update.s3_checksum_algorithm,
        &["SHA256", "ETAG_MD5_COMPATIBLE", "NONE"],
    )?;
    if !(1..=365).contains(&update.s3_multipart_abort_days) {
        bail!("s3MultipartAbortDays must be between 1 and 365");
    }
    validate_policy_enum(
        "s3DefaultEncryptionAlgorithm",
        &update.s3_default_encryption_algorithm,
        &["NONE", "AES256", "aws:kms"],
    )?;
    if let Some(mode) = update.s3_object_lock_default_mode.as_deref() {
        validate_policy_enum(
            "s3ObjectLockDefaultMode",
            mode,
            &["GOVERNANCE", "COMPLIANCE"],
        )?;
    }
    if let Some(days) = update.s3_object_lock_default_retain_days {
        if days < 1 {
            bail!("s3ObjectLockDefaultRetainDays must be positive");
        }
    }
    validate_lifecycle_rules(&update.s3_lifecycle_rules)?;
    validate_s3_resource_policy(&update.s3_resource_policy)?;
    Ok(())
}

fn validate_bucket_policy_defaults(update: &BucketPolicyDefaultsUpdate) -> anyhow::Result<()> {
    validate_hybrid_policy(
        update.access_package_ttl_seconds,
        update.fragment_size_bytes,
        &update.source_selection_strategy,
        &update.fragment_priority_strategy,
        update.failure_threshold,
        &update.fallback_mode,
    )
}

fn validate_hybrid_policy(
    access_package_ttl_seconds: i64,
    fragment_size_bytes: i64,
    source_selection_strategy: &str,
    fragment_priority_strategy: &str,
    failure_threshold: i64,
    fallback_mode: &str,
) -> anyhow::Result<()> {
    if !(60..=3600).contains(&access_package_ttl_seconds) {
        bail!("accessPackageTtlSeconds must be between 60 and 3600");
    }
    if !(1024..=134_217_728).contains(&fragment_size_bytes) {
        bail!("fragmentSizeBytes must be between 1024 and 134217728");
    }
    validate_policy_enum(
        "sourceSelectionStrategy",
        source_selection_strategy,
        &[
            "ORIGIN_REPLICA_EDGE",
            "ORIGIN_ONLY",
            "REPLICA_EDGE_FIRST",
            "PEER_FIRST",
        ],
    )?;
    validate_policy_enum(
        "fragmentPriorityStrategy",
        fragment_priority_strategy,
        &["MANIFEST_ORDER", "INITIAL_FIRST", "RAREST_FIRST"],
    )?;
    if !(1..=20).contains(&failure_threshold) {
        bail!("failureThreshold must be between 1 and 20");
    }
    validate_policy_enum(
        "fallbackMode",
        fallback_mode,
        &["ORIGIN_RANGE", "ORIGIN_FULL_OBJECT", "DISABLED"],
    )?;
    Ok(())
}

#[derive(Debug, Clone)]
struct LifecycleRule {
    prefix: String,
    expire_current_days: Option<i64>,
    abort_incomplete_multipart_days: Option<i64>,
}

fn lifecycle_rules(value: &serde_json::Value) -> anyhow::Result<Vec<LifecycleRule>> {
    let array = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("S3 lifecycle rules must be an array"))?;
    array
        .iter()
        .map(|rule| {
            let status = rule
                .get("Status")
                .or_else(|| rule.get("status"))
                .and_then(|value| value.as_str())
                .unwrap_or("Enabled");
            if status != "Enabled" {
                return Ok(LifecycleRule {
                    prefix: String::new(),
                    expire_current_days: None,
                    abort_incomplete_multipart_days: None,
                });
            }
            let prefix = rule
                .get("Prefix")
                .or_else(|| rule.get("prefix"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_owned();
            let expire_current_days = rule
                .get("Expiration")
                .or_else(|| rule.get("expiration"))
                .and_then(|value| value.get("Days").or_else(|| value.get("days")))
                .and_then(|value| value.as_i64());
            let abort_incomplete_multipart_days = rule
                .get("AbortIncompleteMultipartUpload")
                .or_else(|| rule.get("abortIncompleteMultipartUpload"))
                .and_then(|value| {
                    value
                        .get("DaysAfterInitiation")
                        .or_else(|| value.get("daysAfterInitiation"))
                })
                .and_then(|value| value.as_i64());
            Ok(LifecycleRule {
                prefix,
                expire_current_days,
                abort_incomplete_multipart_days,
            })
        })
        .collect()
}

fn validate_lifecycle_rules(value: &serde_json::Value) -> anyhow::Result<()> {
    for rule in lifecycle_rules(value)? {
        if matches!(rule.expire_current_days, Some(days) if days < 0) {
            bail!("Lifecycle Expiration Days cannot be negative");
        }
        if matches!(rule.abort_incomplete_multipart_days, Some(days) if days < 0) {
            bail!("AbortIncompleteMultipartUpload DaysAfterInitiation cannot be negative");
        }
    }
    Ok(())
}

fn validate_s3_resource_policy(value: &serde_json::Value) -> anyhow::Result<()> {
    let statements = value
        .get("Statement")
        .or_else(|| value.get("statement"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("S3 bucket policy must include a Statement array"))?;
    for statement in statements {
        let effect = statement
            .get("Effect")
            .or_else(|| statement.get("effect"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("S3 bucket policy statements require Effect"))?;
        validate_policy_enum("Effect", effect, &["Allow", "Deny"])?;
    }
    Ok(())
}

fn notifications_include_event(config: &serde_json::Value, event_name: &str) -> bool {
    if config
        .get("EventBridgeEnabled")
        .or_else(|| config.get("eventBridgeEnabled"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        return true;
    }
    config
        .get("Rules")
        .or_else(|| config.get("rules"))
        .and_then(|value| value.as_array())
        .map(|rules| {
            rules.iter().any(|rule| {
                rule.get("Events")
                    .or_else(|| rule.get("events"))
                    .and_then(|value| value.as_array())
                    .map(|events| {
                        events.iter().any(|event| {
                            event
                                .as_str()
                                .map(|event| {
                                    event == event_name
                                        || event == "s3:*"
                                        || (event.ends_with(":*")
                                            && event_name.starts_with(event.trim_end_matches('*')))
                                })
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn validate_s3_object_tags(tags: &[S3ObjectTag]) -> anyhow::Result<()> {
    if tags.len() > 10 {
        bail!("S3 object tagging supports at most 10 tags");
    }
    let mut keys = std::collections::HashSet::new();
    for tag in tags {
        if tag.key.trim().is_empty() || tag.key.len() > 128 {
            bail!("tag key must be between 1 and 128 bytes");
        }
        if tag.value.len() > 256 {
            bail!("tag value must be at most 256 bytes");
        }
        if !keys.insert(tag.key.clone()) {
            bail!("tag keys must be unique");
        }
    }
    Ok(())
}

fn validate_mcp_settings_update(update: &McpSettingsUpdate) -> anyhow::Result<()> {
    if update.endpoint_path != "/mcp" {
        bail!("endpointPath must be /mcp");
    }
    if let Some(bind_host) = &update.bind_host {
        if bind_host.len() > 255 {
            bail!("bindHost is too long");
        }
    }
    if !update.require_auth {
        bail!("MCP authentication cannot be disabled");
    }
    Ok(())
}

fn mcp_audit_event_for_method(method: &str, outcome: &str) -> &'static str {
    if outcome == "failed" && method == "auth" {
        return "MCP_AUTH_FAILED";
    }
    if outcome == "rejected" || outcome == "error" {
        return "MCP_REQUEST_REJECTED";
    }
    if method == "prompts/list" {
        return "MCP_PROMPTS_LISTED";
    }
    if method.starts_with("prompts/") {
        return "MCP_PROMPT_READ";
    }
    if method.starts_with("resources/") {
        return "MCP_RESOURCE_READ";
    }
    if method.starts_with("tools/") {
        return "MCP_TOOL_CALLED";
    }
    "MCP_REQUEST_COMPLETED"
}

fn validate_policy_enum(field: &str, value: &str, allowed: &[&str]) -> anyhow::Result<()> {
    if allowed.iter().any(|allowed_value| *allowed_value == value) {
        return Ok(());
    }
    bail!("{field} is not supported");
}

fn common_prefix_for_key(prefix: &str, delimiter: &str, key: &str) -> Option<String> {
    let rest = key.strip_prefix(prefix)?;
    let delimiter_index = rest.find(delimiter)?;
    Some(format!(
        "{}{}{}",
        prefix,
        &rest[..delimiter_index],
        delimiter
    ))
}

pub fn validate_bucket_name(name: &str) -> anyhow::Result<()> {
    if name.len() < 3 || name.len() > 63 {
        bail!("bucket name must be between 3 and 63 characters");
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.')
    {
        bail!("bucket name may contain only lowercase letters, numbers, dots and hyphens");
    }
    if name.starts_with(['-', '.']) || name.ends_with(['-', '.']) {
        bail!("bucket name cannot start or end with a dot or hyphen");
    }
    if name.contains("..") {
        bail!("bucket name cannot contain consecutive dots");
    }
    if name.parse::<IpAddr>().is_ok() {
        bail!("bucket name cannot be formatted as an IP address");
    }
    Ok(())
}

pub fn validate_object_key(key: &str) -> anyhow::Result<()> {
    if key.trim().is_empty() {
        bail!("object key cannot be empty");
    }
    if key.len() > 1024 {
        bail!("object key cannot exceed 1024 characters");
    }
    if key.contains('\0') {
        bail!("object key cannot contain null bytes");
    }
    if key.contains('\\') {
        bail!("object key cannot contain backslashes");
    }
    let path = Path::new(key);
    if path.is_absolute() {
        bail!("object key must be relative");
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) if !part.is_empty() => {}
            _ => bail!("object key contains an invalid path component"),
        }
    }
    Ok(())
}

fn validate_replica_endpoint(endpoint: &str) -> anyhow::Result<()> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        bail!("replica endpoint cannot be empty");
    }
    if endpoint.len() > 2048 {
        bail!("replica endpoint is too long");
    }
    if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
        bail!("replica endpoint must be an HTTP or HTTPS URL");
    }
    Ok(())
}

fn validate_peer_endpoint(endpoint: &str) -> anyhow::Result<()> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        bail!("peer endpoint cannot be empty");
    }
    if endpoint.len() > 2048 {
        bail!("peer endpoint is too long");
    }
    if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
        bail!("peer endpoint must be an HTTP or HTTPS URL");
    }
    Ok(())
}

fn validate_peer_id(peer_id: &str) -> anyhow::Result<()> {
    if peer_id.trim().is_empty() {
        bail!("peerId cannot be empty");
    }
    if peer_id.len() > 160 {
        bail!("peerId is too long");
    }
    if !peer_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        bail!("peerId contains unsupported characters");
    }
    Ok(())
}

fn validate_source_type(source_type: &str) -> anyhow::Result<()> {
    match source_type {
        "ORIGIN" | "REPLICA_EDGE" | "PEER" => Ok(()),
        _ => bail!("sourceType must be ORIGIN, REPLICA_EDGE or PEER"),
    }
}

fn validate_sdk_event_type(event_type: &str) -> anyhow::Result<()> {
    match event_type {
        "FRAGMENT_VALIDATED" | "FRAGMENT_REJECTED" | "FALLBACK_DECISION" | "SOURCE_FAILURE" => {
            Ok(())
        }
        _ => bail!("eventType is not supported"),
    }
}

fn validate_sdk_outcome(outcome: &str) -> anyhow::Result<()> {
    match outcome {
        "SUCCESS" | "FAILURE" | "REJECTED" => Ok(()),
        _ => bail!("outcome must be SUCCESS, FAILURE or REJECTED"),
    }
}

fn validate_non_negative(value: i64, field: &str) -> anyhow::Result<()> {
    if value < 0 {
        bail!("{field} must be non-negative");
    }
    Ok(())
}

fn validate_health_status(status: &str) -> anyhow::Result<String> {
    let normalized = status.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "OK" | "DEGRADED" | "UNAVAILABLE" => Ok(normalized),
        _ => bail!("status must be OK, DEGRADED or UNAVAILABLE"),
    }
}

#[cfg(test)]
pub fn build_object_manifest(
    bytes: &[u8],
    fragment_size_bytes: i64,
) -> anyhow::Result<NewObjectManifest> {
    if fragment_size_bytes <= 0 {
        bail!("fragmentSizeBytes must be positive");
    }
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    let fragments = bytes
        .chunks(fragment_size)
        .enumerate()
        .map(|(index, chunk)| {
            let start = index
                .checked_mul(fragment_size)
                .and_then(|value| i64::try_from(value).ok())
                .ok_or_else(|| anyhow::anyhow!("fragment byte range is too large"))?;
            let size_bytes =
                i64::try_from(chunk.len()).context("fragment size cannot fit in i64")?;
            let end = start + size_bytes.saturating_sub(1);
            Ok(NewObjectFragment {
                index: i64::try_from(index).context("fragment index cannot fit in i64")?,
                byte_range_start: start,
                byte_range_end: end,
                size_bytes,
                sha256: format!("{:x}", Sha256::digest(chunk)),
                priority: if index == 0 {
                    "INITIAL".to_owned()
                } else {
                    "NORMAL".to_owned()
                },
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(NewObjectManifest {
        fragment_size_bytes,
        fragments,
    })
}

fn split_and_paginate(
    objects: Vec<ObjectSummary>,
    prefix: &str,
    delimiter: Option<&str>,
    page: u32,
    page_size: u32,
) -> (Vec<ObjectSummary>, Vec<String>, i64, u32, u32) {
    let mut common_prefixes: Vec<String> = Vec::new();
    let mut file_items: Vec<ObjectSummary> = Vec::new();
    for object in objects {
        match delimiter.and_then(|delimiter| common_prefix_for_key(prefix, delimiter, &object.key))
        {
            Some(cp) => {
                if !common_prefixes.contains(&cp) {
                    common_prefixes.push(cp);
                }
            }
            None => file_items.push(object),
        }
    }
    let total = i64::try_from(file_items.len()).unwrap_or(i64::MAX);
    let pages = total_pages(total, page_size);
    let clamped_page = page.min(pages).max(1);
    let offset = usize::try_from(clamped_page.saturating_sub(1)).unwrap_or(0)
        * usize::try_from(page_size).unwrap_or(20);
    let limit = usize::try_from(page_size).unwrap_or(20);
    let paged = file_items.into_iter().skip(offset).take(limit).collect();
    (paged, common_prefixes, total, pages, clamped_page)
}

fn map_unique_violation(message: &'static str) -> impl Fn(sqlx_core::Error) -> anyhow::Error {
    move |error| match &error {
        sqlx_core::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            anyhow::anyhow!("{message}")
        }
        _ => anyhow::Error::new(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_pages_never_returns_zero_for_empty_collections() {
        assert_eq!(total_pages(0, 10), 1);
        assert_eq!(total_pages(-1, 10), 1);
    }

    #[test]
    fn total_pages_rounds_up_partial_pages() {
        assert_eq!(total_pages(1, 10), 1);
        assert_eq!(total_pages(10, 10), 1);
        assert_eq!(total_pages(11, 10), 2);
        assert_eq!(total_pages(101, 10), 11);
    }

    #[test]
    fn validate_object_key_rejects_path_traversal_and_unsafe_paths() {
        assert!(validate_object_key("folder/hello.txt").is_ok());
        assert!(validate_object_key("../hello.txt").is_err());
        assert!(validate_object_key("folder/../hello.txt").is_err());
        assert!(validate_object_key("folder\\..\\hello.txt").is_err());
        assert!(validate_object_key("/absolute/hello.txt").is_err());
        assert!(validate_object_key("folder/\0/hello.txt").is_err());
    }

    #[test]
    fn validate_bucket_name_rejects_confusing_or_hostile_names() {
        assert!(validate_bucket_name("game-assets").is_ok());
        assert!(validate_bucket_name("127.0.0.1").is_err());
        assert!(validate_bucket_name("192.168.1.10").is_err());
        assert!(validate_bucket_name("UPPERCASE").is_err());
        assert!(validate_bucket_name("bucket..name").is_err());
        assert!(validate_bucket_name("-bucket").is_err());
        assert!(validate_bucket_name("bucket-").is_err());
    }

    #[test]
    fn validate_mcp_settings_rejects_disabling_authentication() {
        let mut update = secure_mcp_settings_update();
        assert!(validate_mcp_settings_update(&update).is_ok());

        update.endpoint_path = "/invalid".to_string();
        assert!(validate_mcp_settings_update(&update).is_err());
        update.endpoint_path = "/mcp".to_string();

        update.bind_host = Some("a".repeat(256));
        assert!(validate_mcp_settings_update(&update).is_err());
        update.bind_host = Some("127.0.0.1".to_string());

        update.require_auth = false;
        assert!(validate_mcp_settings_update(&update).is_err());
    }

    #[test]
    fn validate_bucket_name_and_object_key_property_testing() {
        let mut seed: u64 = 987654321;
        let mut lcg = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed
        };

        let bucket_chars = "abcdefghijklmnopqrstuvwxyz0123456789-.";
        let bucket_chars_len = bucket_chars.len() as u64;

        for _ in 0..1000 {
            let len = (lcg() % 70) + 1;
            let name: String = (0..len)
                .map(|_| {
                    let idx = (lcg() % bucket_chars_len) as usize;
                    bucket_chars.chars().nth(idx).unwrap()
                })
                .collect();

            let res = validate_bucket_name(&name);
            if res.is_ok() {
                assert!((3..=63).contains(&name.len()));
                assert!(!name.starts_with('-') && !name.starts_with('.'));
                assert!(!name.ends_with('-') && !name.ends_with('.'));
                assert!(!name.contains(".."));
            }
        }

        for _ in 0..1000 {
            let len = lcg() % 1050;
            let key: String = (0..len).map(|_| ((lcg() % 128) as u8) as char).collect();

            let res = validate_object_key(&key);
            if res.is_ok() {
                assert!(!key.trim().is_empty());
                assert!(key.len() <= 1024);
                assert!(!key.contains('\0'));
                assert!(!key.contains('\\'));
                assert!(!key.starts_with('/'));
            }
        }
    }

    #[test]
    fn helper_validations_and_parsers_behave_as_expected() {
        assert!(validate_replica_endpoint("https://replica.local:8443").is_ok());
        assert!(validate_replica_endpoint("ftp://replica.local").is_err());
        assert!(validate_replica_endpoint("").is_err());

        assert!(validate_peer_endpoint("http://peer.local:9000").is_ok());
        assert!(validate_peer_endpoint("invalid-url").is_err());

        assert!(validate_peer_id("peer-node-1").is_ok());
        assert!(validate_peer_id("peer name with spaces").is_err());
        assert!(validate_peer_id("").is_err());

        assert!(validate_source_type("ORIGIN").is_ok());
        assert!(validate_source_type("REPLICA_EDGE").is_ok());
        assert!(validate_source_type("PEER").is_ok());
        assert!(validate_source_type("UNKNOWN").is_err());

        assert!(validate_sdk_event_type("FRAGMENT_VALIDATED").is_ok());
        assert!(validate_sdk_event_type("INVALID").is_err());

        assert!(validate_sdk_outcome("SUCCESS").is_ok());
        assert!(validate_sdk_outcome("INVALID").is_err());

        assert_eq!(validate_health_status("ok").unwrap(), "OK");
        assert!(validate_health_status("BROKEN").is_err());

        assert!(
            parse_fragment_id(
                "m1:0:0000000000000000000000000000000000000000000000000000000000000000"
            )
            .is_ok()
        );
        assert!(parse_fragment_id("invalid:fragment").is_err());
        assert!(
            parse_fragment_id(
                "m1:-1:0000000000000000000000000000000000000000000000000000000000000000"
            )
            .is_err()
        );
        assert!(parse_fragment_id("m1:0:short_hash").is_err());
    }

    #[test]
    fn validate_mcp_scopes_normalizes_and_rejects_unknown_scopes() {
        assert_eq!(
            validate_mcp_scopes(&[" admin ".to_string()]).expect("scopes"),
            vec!["read".to_string(), "admin".to_string(), "write".to_string()]
        );
        assert!(validate_mcp_scopes(&["root".to_string()]).is_err());
    }

    #[test]
    fn application_scopes_are_closed_and_credential_names_are_bounded() {
        assert_eq!(
            validate_application_scopes(&[
                " pontemesh:manifest:read ".to_owned(),
                "PONTEMESH:MANIFEST:READ".to_owned(),
            ])
            .expect("valid application scopes"),
            vec!["pontemesh:manifest:read"]
        );
        assert!(validate_application_scopes(&["*".to_owned()]).is_err());
        assert!(validate_application_scopes(&["unknown".to_owned()]).is_err());
        assert!(validate_credential_name(&"a".repeat(256), "token name").is_err());
    }

    #[test]
    fn object_manifest_fragments_cover_object_ranges_and_hashes() {
        let manifest = build_object_manifest(b"abcdef", 2).expect("manifest");

        assert_eq!(manifest.fragment_size_bytes, 2);
        assert_eq!(manifest.fragments.len(), 3);
        assert_eq!(manifest.fragments[0].index, 0);
        assert_eq!(manifest.fragments[0].byte_range_start, 0);
        assert_eq!(manifest.fragments[0].byte_range_end, 1);
        assert_eq!(manifest.fragments[0].size_bytes, 2);
        assert_eq!(manifest.fragments[0].priority, "INITIAL");
        assert_eq!(
            manifest.fragments[0].sha256,
            format!("{:x}", Sha256::digest(b"ab"))
        );
        assert_eq!(manifest.fragments[2].byte_range_start, 4);
        assert_eq!(manifest.fragments[2].byte_range_end, 5);
        assert_eq!(manifest.fragments[2].priority, "NORMAL");
    }

    #[test]
    fn object_manifest_rejects_invalid_fragment_size() {
        assert!(build_object_manifest(b"abc", 0).is_err());
        assert!(build_object_manifest(b"abc", -1).is_err());
    }

    fn secure_mcp_settings_update() -> McpSettingsUpdate {
        McpSettingsUpdate {
            enabled: true,
            endpoint_path: "/mcp".to_string(),
            bind_host: Some("127.0.0.1".to_string()),
            require_auth: true,
            read_tools_enabled: true,
            write_tools_enabled: false,
            admin_tools_enabled: false,
            expose_resources: true,
            expose_prompts: true,
            allow_localhost_only: true,
        }
    }

    fn make_object(key: &str) -> ObjectSummary {
        ObjectSummary {
            key: key.to_string(),
            size_bytes: 100,
            content_type: "application/octet-stream".to_string(),
            sha256: "abc".to_string(),
            version_id: None,
            is_delete_marker: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            state: "AVAILABLE".to_string(),
            created_by: None,
        }
    }

    #[test]
    fn common_prefix_for_key_returns_none_when_no_delimiter_in_rest() {
        assert_eq!(common_prefix_for_key("", "/", "file.txt"), None);
        assert_eq!(common_prefix_for_key("a/", "/", "a/file.txt"), None);
    }

    #[test]
    fn common_prefix_for_key_returns_prefix_up_to_and_including_delimiter() {
        assert_eq!(
            common_prefix_for_key("", "/", "folder/file.txt"),
            Some("folder/".to_string())
        );
        assert_eq!(
            common_prefix_for_key("a/", "/", "a/b/file.txt"),
            Some("a/b/".to_string())
        );
        assert_eq!(
            common_prefix_for_key("a/", "/", "a/b/c/file.txt"),
            Some("a/b/".to_string())
        );
    }

    #[test]
    fn common_prefix_for_key_returns_none_when_key_does_not_start_with_prefix() {
        assert_eq!(common_prefix_for_key("x/", "/", "y/file.txt"), None);
    }

    #[test]
    fn split_and_paginate_separates_files_from_subdirectories() {
        let objects = vec![
            make_object("docs/readme.md"),
            make_object("docs/guide.md"),
            make_object("images/logo.png"),
            make_object("root.txt"),
        ];
        let (files, prefixes, total, _pages, _page) =
            split_and_paginate(objects, "", Some("/"), 1, 20);

        assert_eq!(total, 1);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].key, "root.txt");
        assert_eq!(prefixes.len(), 2);
        assert!(prefixes.contains(&"docs/".to_string()));
        assert!(prefixes.contains(&"images/".to_string()));
    }

    #[test]
    fn split_and_paginate_deduplicates_common_prefixes() {
        let objects = vec![
            make_object("a/file1.txt"),
            make_object("a/file2.txt"),
            make_object("a/file3.txt"),
        ];
        let (_files, prefixes, _total, _pages, _page) =
            split_and_paginate(objects, "", Some("/"), 1, 20);

        assert_eq!(prefixes, vec!["a/".to_string()]);
    }

    #[test]
    fn split_and_paginate_paginates_file_items_only() {
        let objects: Vec<ObjectSummary> = (0..25)
            .map(|i| make_object(&format!("file{i:02}.txt")))
            .collect();

        let (page1, _, total, total_pages, page_num) =
            split_and_paginate(objects.clone(), "", Some("/"), 1, 10);
        assert_eq!(total, 25);
        assert_eq!(total_pages, 3);
        assert_eq!(page_num, 1);
        assert_eq!(page1.len(), 10);
        assert_eq!(page1[0].key, "file00.txt");

        let (page3, _, _, _, _) = split_and_paginate(objects, "", Some("/"), 3, 10);
        assert_eq!(page3.len(), 5);
        assert_eq!(page3[0].key, "file20.txt");
    }

    #[test]
    fn split_and_paginate_clamps_page_to_last_when_out_of_bounds() {
        let objects: Vec<ObjectSummary> = (0..5)
            .map(|i| make_object(&format!("file{i}.txt")))
            .collect();

        let (items, _, _, _, page_num) = split_and_paginate(objects, "", Some("/"), 99, 10);
        assert_eq!(page_num, 1);
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn split_and_paginate_with_prefix_only_counts_direct_children() {
        let objects = vec![make_object("a/b/deep.txt"), make_object("a/direct.txt")];
        let (files, prefixes, total, _, _) = split_and_paginate(objects, "a/", Some("/"), 1, 20);

        assert_eq!(total, 1);
        assert_eq!(files[0].key, "a/direct.txt");
        assert_eq!(prefixes, vec!["a/b/".to_string()]);
    }

    #[test]
    fn split_and_paginate_without_delimiter_keeps_nested_keys_as_items() {
        let objects = vec![make_object("docs/readme.md"), make_object("root.txt")];
        let (files, prefixes, total, _, _) = split_and_paginate(objects, "", None, 1, 20);

        assert_eq!(total, 2);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].key, "docs/readme.md");
        assert_eq!(files[1].key, "root.txt");
        assert!(prefixes.is_empty());
    }
}

fn validate_mcp_scopes(scopes: &[String]) -> anyhow::Result<Vec<String>> {
    let mut normalized = Vec::new();
    for scope in scopes {
        let scope = scope.trim().to_ascii_lowercase();
        if !matches!(scope.as_str(), "read" | "write" | "admin") {
            bail!("invalid MCP token scope: {scope}");
        }
        if !normalized.contains(&scope) {
            normalized.push(scope);
        }
    }
    if normalized.is_empty() {
        normalized.push("read".to_owned());
    }
    if normalized.iter().any(|s| s == "admin") && !normalized.iter().any(|s| s == "write") {
        normalized.push("write".to_owned());
    }
    if !normalized.iter().any(|s| s == "read") {
        normalized.insert(0, "read".to_owned());
    }
    Ok(normalized)
}

fn validate_application_scopes(scopes: &[String]) -> anyhow::Result<Vec<String>> {
    const ALLOWED: [&str; 7] = [
        "origin:objects:read",
        "origin:objects:write",
        "pontemesh:access-package:create",
        "pontemesh:manifest:read",
        "pontemesh:sources:read",
        "pontemesh:availability:read",
        "pontemesh:policies:read",
    ];

    let mut normalized = Vec::new();
    for scope in scopes {
        let scope = scope.trim().to_ascii_lowercase();
        if !ALLOWED.contains(&scope.as_str()) {
            bail!("invalid application scope: {scope}");
        }
        if !normalized.contains(&scope) {
            normalized.push(scope);
        }
    }

    if normalized.is_empty() {
        bail!("application credential must include at least one scope");
    }
    Ok(normalized)
}

fn validate_credential_name(name: &str, field: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        bail!("{field} cannot be empty");
    }
    if name.chars().count() > 255 {
        bail!("{field} cannot exceed 255 characters");
    }
    Ok(())
}
