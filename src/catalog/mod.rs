use crate::{
    config,
    security::{random::secure_url_token, token::hash_bearer_token},
};
use anyhow::{Context, bail};
use serde::Serialize;
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
pub struct ObjectSummary {
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub created_at: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct ObjectRecord {
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub storage_path: String,
    pub created_at: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTotals {
    pub total_buckets: i64,
    pub total_objects: i64,
    pub total_object_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketPolicy {
    pub bucket_name: String,
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct BucketPolicyUpdate {
    pub access_package_ttl_seconds: i64,
    pub fragment_size_bytes: i64,
    pub allow_replica_edge: bool,
    pub allow_peer_sharing: bool,
}

#[derive(Debug, Clone)]
pub struct NewObject {
    pub bucket_name: String,
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub storage_path: String,
    pub manifest: NewObjectManifest,
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
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub state: String,
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
    pub fragment_events: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTrafficMetric {
    pub bucket: String,
    pub key: String,
    pub origin_bytes_served: i64,
    pub origin_requests: i64,
    pub replica_bytes_synced: i64,
    pub fragment_events: i64,
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
pub struct OriginTrafficSummary {
    pub total_requests: i64,
    pub full_object_requests: i64,
    pub range_requests: i64,
    pub total_bytes_served: i64,
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
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
                COUNT(o.id)::bigint AS object_count,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_id = b.id AND o.deleted_at IS NULL
            LEFT JOIN object_versions v
                ON v.id = o.current_version_id
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

    pub async fn get_bucket(&self, name: &str) -> anyhow::Result<Option<BucketSummary>> {
        validate_bucket_name(name)?;
        let row = query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(o.id)::bigint AS object_count,
                COALESCE(SUM(v.size_bytes), 0)::bigint AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_id = b.id AND o.deleted_at IS NULL
            LEFT JOIN object_versions v
                ON v.id = o.current_version_id
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
            INSERT INTO bucket_policies (bucket_id)
            VALUES ($1::uuid)
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
                allow_replica_edge, allow_peer_sharing, updated_at
            )
            VALUES ($1::uuid, $2, $3, $4, $5, now())
            ON CONFLICT (bucket_id) DO UPDATE SET
                access_package_ttl_seconds = EXCLUDED.access_package_ttl_seconds,
                fragment_size_bytes = EXCLUDED.fragment_size_bytes,
                allow_replica_edge = EXCLUDED.allow_replica_edge,
                allow_peer_sharing = EXCLUDED.allow_peer_sharing,
                updated_at = now()
            RETURNING access_package_ttl_seconds, fragment_size_bytes,
                allow_replica_edge, allow_peer_sharing, updated_at
            "#,
        )
        .bind(bucket_id)
        .bind(update.access_package_ttl_seconds)
        .bind(update.fragment_size_bytes)
        .bind(update.allow_replica_edge)
        .bind(update.allow_peer_sharing)
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
            updated_at: format_datetime(row.get("updated_at")),
        })
    }

    pub async fn list_objects(&self, bucket_name: &str) -> anyhow::Result<Vec<ObjectSummary>> {
        validate_bucket_name(bucket_name)?;
        let rows = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash,
                v.created_at, o.state
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE b.name = $1 AND b.deleted_at IS NULL AND o.deleted_at IS NULL
            ORDER BY v.created_at DESC, o.object_key ASC
            "#,
        )
        .bind(bucket_name)
        .fetch_all(&self.pool)
        .await
        .context("failed to list objects")?;

        Ok(rows.into_iter().map(object_summary_from_row).collect())
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

        let version_row = query(
            r#"
            INSERT INTO object_versions (
                object_id, version_number, size_bytes, content_type,
                hash_algorithm, object_hash, storage_path
            )
            VALUES ($1::uuid, $2, $3, $4, 'SHA-256', $5, $6)
            RETURNING id::text, created_at
            "#,
        )
        .bind(&object_id)
        .bind(version_number)
        .bind(object.size_bytes)
        .bind(&object.content_type)
        .bind(&object.sha256)
        .bind(&object.storage_path)
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
            created_at: format_datetime(version_row.get("created_at")),
            state: "AVAILABLE".to_owned(),
        })
    }

    pub async fn get_object(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<ObjectSummary>> {
        Ok(self
            .get_object_record(bucket_name, object_key)
            .await?
            .map(|record| ObjectSummary {
                key: record.key,
                size_bytes: record.size_bytes,
                content_type: record.content_type,
                sha256: record.sha256,
                created_at: record.created_at,
                state: record.state,
            }))
    }

    pub async fn get_object_record(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<ObjectRecord>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = query(
            r#"
            SELECT o.object_key, v.size_bytes, v.content_type, v.object_hash,
                v.storage_path, v.created_at, o.state
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            WHERE b.name = $1 AND o.object_key = $2
              AND b.deleted_at IS NULL AND o.deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load object")?;

        Ok(row.map(object_record_from_row))
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
                m.created_at
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

    pub async fn delete_object(&self, bucket_name: &str, object_key: &str) -> anyhow::Result<()> {
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
        if name.is_empty() {
            bail!("application name cannot be empty");
        }
        if scopes.is_empty() {
            bail!("application credential must include at least one scope");
        }

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
        if name.is_empty() {
            bail!("replica name cannot be empty");
        }
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
            SELECT id::text, name, allowed_buckets
            FROM replica_credentials
            WHERE token_hash = $1 AND revoked_at IS NULL
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
                   v.created_at, o.state
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
                   v.object_hash, v.storage_path, v.created_at, o.state,
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
                created_at: format_datetime(row.get("created_at")),
                state: row.get("state"),
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
            SELECT b.name, o.object_key, v.size_bytes, v.content_type, v.object_hash, o.state
            FROM objects o
            JOIN buckets b ON b.id = o.bucket_id
            JOIN object_versions v ON v.id = o.current_version_id
            JOIN bucket_policies p ON p.bucket_id = b.id
            WHERE b.name = ANY($1)
              AND b.deleted_at IS NULL
              AND o.deleted_at IS NULL
              AND o.state = 'AVAILABLE'
              AND p.allow_replica_edge = TRUE
            ORDER BY v.created_at DESC, o.object_key ASC
            "#,
        )
        .bind(allowed_buckets)
        .fetch_all(&self.pool)
        .await
        .context("failed to list replica sync objects")?;

        Ok(rows
            .into_iter()
            .map(|row| ReplicaSyncObject {
                bucket: row.get("name"),
                key: row.get("object_key"),
                size_bytes: row.get("size_bytes"),
                content_type: row.get("content_type"),
                sha256: row.get("object_hash"),
                state: row.get("state"),
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
            ORDER BY a.last_seen_at DESC, r.name ASC
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_all(&self.pool)
        .await
        .context("failed to list authorized replica sources")?;

        rows.into_iter().map(availability_record_from_row).collect()
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
              AND r.revoked_at IS NULL
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
        })
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
                COALESCE(SUM(bytes_served), 0)::bigint AS total_bytes_served
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
                       COUNT(*)::bigint AS fragment_events
                FROM fragment_transfer_events
                GROUP BY bucket_id
            )
            SELECT
                b.name AS bucket_name,
                COALESCE(origin.bytes_served, 0)::bigint AS origin_bytes_served,
                COALESCE(origin.requests, 0)::bigint AS origin_requests,
                COALESCE(fragments.replica_bytes_synced, 0)::bigint AS replica_bytes_synced,
                COALESCE(fragments.fragment_events, 0)::bigint AS fragment_events
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
                fragment_events: row.get("fragment_events"),
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
                COALESCE(fragments.fragment_events, 0)::bigint AS fragment_events
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
                fragment_events: row.get("fragment_events"),
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
        created_at: format_datetime(row.get("created_at")),
        state: row.get("state"),
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
        created_at: format_datetime(row.get("created_at")),
        state: row.get("state"),
    }
}

fn bucket_policy_from_row(row: PgRow) -> BucketPolicy {
    BucketPolicy {
        bucket_name: row.get("name"),
        access_package_ttl_seconds: row.get("access_package_ttl_seconds"),
        fragment_size_bytes: row.get("fragment_size_bytes"),
        allow_replica_edge: row.get("allow_replica_edge"),
        allow_peer_sharing: row.get("allow_peer_sharing"),
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
    if !(60..=3600).contains(&update.access_package_ttl_seconds) {
        bail!("accessPackageTtlSeconds must be between 60 and 3600");
    }
    if !(1024..=134_217_728).contains(&update.fragment_size_bytes) {
        bail!("fragmentSizeBytes must be between 1024 and 134217728");
    }
    Ok(())
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
}
