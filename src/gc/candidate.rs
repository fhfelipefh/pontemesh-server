use anyhow::Context;
use sqlx::PgPool;
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row};
use tracing::warn;

pub const REASON_OBJECT_DELETED: &str = "OBJECT_DELETED";
pub const REASON_ABORTED_MULTIPART: &str = "ABORTED_MULTIPART";
pub const REASON_TEMP_FILE: &str = "TEMP_FILE";

pub const STATE_PENDING: &str = "PENDING";
pub const STATE_SWEEPING: &str = "SWEEPING";
pub const STATE_DELETED: &str = "DELETED";
pub const STATE_FAILED: &str = "FAILED";

#[derive(Debug, Clone)]
pub struct GcCandidate {
    pub id: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub storage_path: Option<String>,
    pub reason: String,
    pub state: String,
    pub attempt_count: i32,
    pub sweep_token: Option<String>,
}

pub async fn enqueue_deleted_object(
    pool: &PgPool,
    version_id: &str,
    storage_path: &str,
    grace_seconds: u64,
) -> anyhow::Result<()> {
    let grace = i64::try_from(grace_seconds).unwrap_or(7200);
    query(
        r#"
        INSERT INTO gc_candidates (resource_type, resource_id, storage_path, reason, not_before)
        VALUES ('OBJECT_VERSION', $1::uuid, $2, $3, now() + ($4 * interval '1 second'))
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(version_id)
    .bind(storage_path)
    .bind(REASON_OBJECT_DELETED)
    .bind(grace)
    .execute(pool)
    .await
    .context("failed to enqueue gc candidate")?;
    Ok(())
}

pub async fn enqueue_aborted_multipart_parts(
    pool: &PgPool,
    upload_id: &str,
    grace_seconds: u64,
) -> anyhow::Result<()> {
    let grace = i64::try_from(grace_seconds).unwrap_or(7200);
    let rows =
        query("SELECT storage_path FROM s3_multipart_upload_parts WHERE upload_id = $1::uuid")
            .bind(upload_id)
            .fetch_all(pool)
            .await
            .context("failed to list multipart parts for gc")?;

    for row in rows {
        let path: String = row.get("storage_path");
        if let Err(error) = query(
            r#"
            INSERT INTO gc_candidates (resource_type, storage_path, reason, not_before)
            VALUES ('MULTIPART_PART', $1, $2, now() + ($3 * interval '1 second'))
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&path)
        .bind(REASON_ABORTED_MULTIPART)
        .bind(grace)
        .execute(pool)
        .await
        {
            warn!(upload_id, path, error = %error, "gc: failed to enqueue multipart part");
        }
    }
    Ok(())
}

pub async fn claim_batch(
    pool: &PgPool,
    batch_size: i64,
    lease_seconds: u64,
) -> anyhow::Result<Vec<GcCandidate>> {
    let lease = i64::try_from(lease_seconds).unwrap_or(300);
    let sweep_token = uuid::Uuid::new_v4().to_string();
    let rows = query(
        r#"
        UPDATE gc_candidates
        SET state = $1,
            sweep_token = $2::uuid,
            sweep_lease_until = now() + ($3 * interval '1 second'),
            attempt_count = attempt_count + 1,
            last_attempt_at = now()
        WHERE id IN (
            SELECT id FROM gc_candidates
            WHERE state = $4
              AND not_before <= now()
              AND deleted_at IS NULL
            ORDER BY not_before
            FOR UPDATE SKIP LOCKED
            LIMIT $5
        )
        RETURNING id::text, resource_type, resource_id::text, storage_path,
                  reason, state, attempt_count, sweep_token::text
        "#,
    )
    .bind(STATE_SWEEPING)
    .bind(&sweep_token)
    .bind(lease)
    .bind(STATE_PENDING)
    .bind(batch_size)
    .fetch_all(pool)
    .await
    .context("failed to claim gc batch")?;

    Ok(rows
        .into_iter()
        .map(|row| GcCandidate {
            id: row.get("id"),
            resource_type: row.get("resource_type"),
            resource_id: row.get("resource_id"),
            storage_path: row.get("storage_path"),
            reason: row.get("reason"),
            state: row.get("state"),
            attempt_count: row.get("attempt_count"),
            sweep_token: row.get("sweep_token"),
        })
        .collect())
}

pub async fn mark_deleted(pool: &PgPool, id: &str, sweep_token: &str) -> anyhow::Result<()> {
    query(
        r#"
        UPDATE gc_candidates
        SET state = $1, deleted_at = now()
        WHERE id = $2::uuid AND sweep_token = $3::uuid
        "#,
    )
    .bind(STATE_DELETED)
    .bind(id)
    .bind(sweep_token)
    .execute(pool)
    .await
    .context("failed to mark gc candidate deleted")?;
    Ok(())
}

pub async fn mark_failed(
    pool: &PgPool,
    id: &str,
    max_retries: u32,
    error_msg: &str,
) -> anyhow::Result<()> {
    let attempts: i32 = query_scalar("SELECT attempt_count FROM gc_candidates WHERE id = $1::uuid")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("failed to check attempt count")?
        .unwrap_or(0);

    let next_state = if attempts as u32 >= max_retries {
        STATE_FAILED
    } else {
        STATE_PENDING
    };

    query(
        r#"
        UPDATE gc_candidates
        SET state = $1, last_error = $2, sweep_token = NULL, sweep_lease_until = NULL
        WHERE id = $3::uuid
        "#,
    )
    .bind(next_state)
    .bind(error_msg)
    .bind(id)
    .execute(pool)
    .await
    .context("failed to mark gc candidate failed")?;
    Ok(())
}

pub async fn reclaim_expired_leases(pool: &PgPool) -> anyhow::Result<u64> {
    let result = query(
        r#"
        UPDATE gc_candidates
        SET state = $1, sweep_token = NULL, sweep_lease_until = NULL
        WHERE state = $2 AND sweep_lease_until < now()
        "#,
    )
    .bind(STATE_PENDING)
    .bind(STATE_SWEEPING)
    .execute(pool)
    .await
    .context("failed to reclaim expired gc leases")?;
    Ok(result.rows_affected())
}

pub async fn pending_stats(pool: &PgPool) -> anyhow::Result<(i64, i64)> {
    let row = query(
        r#"
        SELECT
            COUNT(*)::bigint AS cnt,
            COALESCE(SUM(
                CASE WHEN c.resource_id IS NOT NULL THEN
                    (SELECT v.size_bytes FROM object_versions v WHERE v.id = c.resource_id LIMIT 1)
                ELSE 0 END
            ), 0)::bigint AS bytes
        FROM gc_candidates c
        WHERE c.state = $1 AND c.deleted_at IS NULL
        "#,
    )
    .bind(STATE_PENDING)
    .fetch_one(pool)
    .await
    .context("failed to count gc pending")?;

    Ok((row.get("cnt"), row.get("bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_candidate_struct_instantiation() {
        let candidate = GcCandidate {
            id: "cand-1".to_string(),
            resource_type: "OBJECT_VERSION".to_string(),
            resource_id: Some("res-123".to_string()),
            storage_path: Some("/tmp/storage/1".to_string()),
            reason: REASON_OBJECT_DELETED.to_string(),
            state: STATE_PENDING.to_string(),
            attempt_count: 0,
            sweep_token: None,
        };

        assert_eq!(candidate.id, "cand-1");
        assert_eq!(candidate.reason, "OBJECT_DELETED");
        assert_eq!(candidate.state, "PENDING");
    }
}
