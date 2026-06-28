use crate::config::PontemeshHome;
use anyhow::{Context, bail};
use serde::Serialize;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{path::Path, str::FromStr};

#[derive(Debug, Clone)]
pub struct Catalog {
    pool: SqlitePool,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTotals {
    pub total_buckets: i64,
    pub total_objects: i64,
    pub total_object_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct NewObject {
    pub bucket_name: String,
    pub key: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub sha256: String,
    pub storage_path: String,
}

impl Catalog {
    pub async fn initialize(paths: &PontemeshHome) -> anyhow::Result<Self> {
        let database_file = paths.catalog_database_file();
        if let Some(parent) = database_file.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create catalog directory {}", parent.display())
            })?;
        }

        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}", database_file.display()))?
                .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| {
                format!(
                    "failed to open catalog database {}",
                    database_file.display()
                )
            })?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS buckets (
                name TEXT PRIMARY KEY,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await
        .context("failed to migrate buckets table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                bucket_name TEXT NOT NULL,
                key TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                content_type TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                storage_path TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                deleted_at TEXT,
                FOREIGN KEY(bucket_name) REFERENCES buckets(name)
            );
            "#,
        )
        .execute(&pool)
        .await
        .context("failed to migrate objects table")?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_active_key ON objects(bucket_name, key) WHERE deleted_at IS NULL;",
        )
        .execute(&pool)
        .await
        .context("failed to migrate objects unique index")?;

        Ok(Self { pool })
    }

    pub async fn database_connected(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    pub async fn list_buckets(&self) -> anyhow::Result<Vec<BucketSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(o.id) AS object_count,
                COALESCE(SUM(o.size_bytes), 0) AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_name = b.name AND o.deleted_at IS NULL
            GROUP BY b.name, b.created_at
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
                created_at: row.get("created_at"),
                object_count: row.get("object_count"),
                total_bytes: row.get("total_bytes"),
            })
            .collect())
    }

    pub async fn get_bucket(&self, name: &str) -> anyhow::Result<Option<BucketSummary>> {
        validate_bucket_name(name)?;
        let row = sqlx::query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(o.id) AS object_count,
                COALESCE(SUM(o.size_bytes), 0) AS total_bytes
            FROM buckets b
            LEFT JOIN objects o
                ON o.bucket_name = b.name AND o.deleted_at IS NULL
            WHERE b.name = ?
            GROUP BY b.name, b.created_at
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load bucket")?;

        Ok(row.map(|row| BucketSummary {
            name: row.get("name"),
            created_at: row.get("created_at"),
            object_count: row.get("object_count"),
            total_bytes: row.get("total_bytes"),
        }))
    }

    pub async fn create_bucket(&self, name: &str) -> anyhow::Result<BucketSummary> {
        validate_bucket_name(name)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query("INSERT INTO buckets (name, created_at) VALUES (?, ?)")
            .bind(name)
            .bind(&created_at)
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => Ok(BucketSummary {
                name: name.to_owned(),
                object_count: 0,
                total_bytes: 0,
                created_at,
            }),
            Err(error) if is_unique_violation(&error) => bail!("bucket already exists: {name}"),
            Err(error) => Err(error).context("failed to create bucket"),
        }
    }

    pub async fn delete_bucket(&self, name: &str) -> anyhow::Result<()> {
        validate_bucket_name(name)?;
        let object_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM objects WHERE bucket_name = ? AND deleted_at IS NULL",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .context("failed to count bucket objects")?;
        if object_count > 0 {
            bail!("bucket must be empty before it can be deleted");
        }

        sqlx::query("DELETE FROM objects WHERE bucket_name = ? AND deleted_at IS NOT NULL")
            .bind(name)
            .execute(&self.pool)
            .await
            .context("failed to prune deleted bucket objects")?;

        let result = sqlx::query("DELETE FROM buckets WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .context("failed to delete bucket")?;
        if result.rows_affected() == 0 {
            bail!("bucket not found: {name}");
        }
        Ok(())
    }

    pub async fn list_objects(&self, bucket_name: &str) -> anyhow::Result<Vec<ObjectSummary>> {
        validate_bucket_name(bucket_name)?;
        let rows = sqlx::query(
            r#"
            SELECT key, size_bytes, content_type, sha256, created_at, state
            FROM objects
            WHERE bucket_name = ? AND deleted_at IS NULL
            ORDER BY created_at DESC, key ASC
            "#,
        )
        .bind(bucket_name)
        .fetch_all(&self.pool)
        .await
        .context("failed to list objects")?;

        Ok(rows
            .into_iter()
            .map(|row| ObjectSummary {
                key: row.get("key"),
                size_bytes: row.get("size_bytes"),
                content_type: row.get("content_type"),
                sha256: row.get("sha256"),
                created_at: row.get("created_at"),
                state: row.get("state"),
            })
            .collect())
    }

    pub async fn insert_object(&self, object: NewObject) -> anyhow::Result<ObjectSummary> {
        validate_bucket_name(&object.bucket_name)?;
        validate_object_key(&object.key)?;
        if self.get_bucket(&object.bucket_name).await?.is_none() {
            bail!("bucket not found: {}", object.bucket_name);
        }

        let created_at = chrono::Utc::now().to_rfc3339();
        let state = "AVAILABLE";
        let result = sqlx::query(
            r#"
            INSERT INTO objects (
                id, bucket_name, key, size_bytes, content_type, sha256,
                storage_path, state, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&object.bucket_name)
        .bind(&object.key)
        .bind(object.size_bytes)
        .bind(&object.content_type)
        .bind(&object.sha256)
        .bind(&object.storage_path)
        .bind(state)
        .bind(&created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(ObjectSummary {
                key: object.key,
                size_bytes: object.size_bytes,
                content_type: object.content_type,
                sha256: object.sha256,
                created_at,
                state: state.to_owned(),
            }),
            Err(error) if is_unique_violation(&error) => {
                bail!("object already exists in bucket: {}", object.key)
            }
            Err(error) => Err(error).context("failed to register object"),
        }
    }

    pub async fn get_object(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> anyhow::Result<Option<ObjectSummary>> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let row = sqlx::query(
            r#"
            SELECT key, size_bytes, content_type, sha256, created_at, state
            FROM objects
            WHERE bucket_name = ? AND key = ? AND deleted_at IS NULL
            "#,
        )
        .bind(bucket_name)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load object")?;

        Ok(row.map(|row| ObjectSummary {
            key: row.get("key"),
            size_bytes: row.get("size_bytes"),
            content_type: row.get("content_type"),
            sha256: row.get("sha256"),
            created_at: row.get("created_at"),
            state: row.get("state"),
        }))
    }

    pub async fn delete_object(&self, bucket_name: &str, object_key: &str) -> anyhow::Result<()> {
        validate_bucket_name(bucket_name)?;
        validate_object_key(object_key)?;
        let deleted_at = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE objects SET state = 'DELETED', deleted_at = ? WHERE bucket_name = ? AND key = ? AND deleted_at IS NULL",
        )
        .bind(deleted_at)
        .bind(bucket_name)
        .bind(object_key)
        .execute(&self.pool)
        .await
        .context("failed to delete object")?;
        if result.rows_affected() == 0 {
            bail!("object not found: {object_key}");
        }
        Ok(())
    }

    pub async fn totals(&self) -> anyhow::Result<ObjectTotals> {
        let total_buckets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buckets")
            .fetch_one(&self.pool)
            .await
            .context("failed to count buckets")?;
        let row = sqlx::query(
            "SELECT COUNT(*) AS total_objects, COALESCE(SUM(size_bytes), 0) AS total_object_bytes FROM objects WHERE deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to count objects")?;

        Ok(ObjectTotals {
            total_buckets,
            total_objects: row.get("total_objects"),
            total_object_bytes: row.get("total_object_bytes"),
        })
    }
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

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database_error) if database_error.is_unique_violation())
}
