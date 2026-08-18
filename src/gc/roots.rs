use anyhow::Context;
use sqlx::PgPool;
use sqlx_core::{query::query, row::Row};

pub async fn is_version_reachable(pool: &PgPool, version_id: &str) -> anyhow::Result<bool> {
    let row = query(
        r#"
        SELECT
            (
                o.current_version_id = v.id
                OR v.legal_hold = TRUE
                OR (v.retain_until IS NOT NULL AND v.retain_until > now())
                OR EXISTS (
                    SELECT 1 FROM access_packages ap
                    WHERE ap.object_manifest_id = m.id
                      AND ap.expires_at > now()
                      AND ap.revoked_at IS NULL
                )
            ) AS reachable
        FROM object_versions v
        JOIN objects o ON o.id = v.object_id
        LEFT JOIN object_manifests m ON m.object_version_id = v.id
        WHERE v.id = $1::uuid
          AND o.deleted_at IS NULL
        "#,
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("failed to check version reachability")?;

    Ok(row.map_or(false, |r| r.get::<bool, _>("reachable")))
}
