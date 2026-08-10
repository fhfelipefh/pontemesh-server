use crate::gc::candidate::{GcCandidate, mark_deleted, mark_failed};
use crate::gc::roots::is_version_reachable;
use anyhow::Context;
use sqlx::PgPool;
use sqlx_core::query::query;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub async fn sweep_candidate(
    pool: &PgPool,
    candidate: &GcCandidate,
    storage_dir: &Path,
    quarantine_dir: &Path,
    max_retries: u32,
) -> anyhow::Result<Option<u64>> {
    if let Some(version_id) = candidate.resource_id.as_deref() {
        match is_version_reachable(pool, version_id).await {
            Ok(true) => {
                query(
                    "UPDATE gc_candidates SET state = 'PROTECTED', sweep_token = NULL, sweep_lease_until = NULL WHERE id = $1::uuid",
                )
                .bind(&candidate.id)
                .execute(pool)
                .await
                .context("failed to mark gc candidate protected")?;
                return Ok(None);
            }
            Err(error) => {
                warn!(id = %candidate.id, error = %error, "gc reachability check failed");
                mark_failed(pool, &candidate.id, max_retries, &error.to_string()).await?;
                return Ok(None);
            }
            Ok(false) => {}
        }
    }

    let Some(storage_path) = candidate.storage_path.as_deref() else {
        let token = candidate.sweep_token.as_deref().unwrap_or("");
        mark_deleted(pool, &candidate.id, token).await?;
        return Ok(Some(0));
    };

    let physical = PathBuf::from(storage_path);
    if !physical.starts_with(storage_dir) {
        warn!(
            id = %candidate.id,
            path = storage_path,
            "gc candidate path outside storage dir, skipping"
        );
        mark_failed(pool, &candidate.id, max_retries, "path outside storage dir").await?;
        return Ok(None);
    }

    if !physical.exists() {
        info!(id = %candidate.id, path = storage_path, "gc: file already absent");
        let token = candidate.sweep_token.as_deref().unwrap_or("");
        mark_deleted(pool, &candidate.id, token).await?;
        return Ok(Some(0));
    }

    let file_size = tokio::fs::metadata(&physical).await.map(|m| m.len()).unwrap_or(0);

    let quarantine_path = quarantine_dir.join(format!(
        "{}-{}",
        candidate.id,
        physical.file_name().and_then(|n| n.to_str()).unwrap_or("blob")
    ));

    if let Some(parent) = quarantine_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }

    if let Err(error) = tokio::fs::rename(&physical, &quarantine_path).await {
        let msg = format!("failed to quarantine: {error}");
        warn!(id = %candidate.id, error = %msg, "gc quarantine failed");
        mark_failed(pool, &candidate.id, max_retries, &msg).await?;
        return Ok(None);
    }

    query(
        "UPDATE gc_candidates SET quarantine_path = $1, quarantined_at = now() WHERE id = $2::uuid",
    )
    .bind(quarantine_path.display().to_string())
    .bind(&candidate.id)
    .execute(pool)
    .await
    .context("failed to record quarantine")?;

    info!(
        id = %candidate.id,
        path = storage_path,
        quarantine = %quarantine_path.display(),
        bytes = file_size,
        "gc: file quarantined"
    );

    let token = candidate.sweep_token.as_deref().unwrap_or("");
    mark_deleted(pool, &candidate.id, token).await?;
    Ok(Some(file_size))
}

pub async fn purge_quarantine(quarantine_dir: &Path, max_age_seconds: u64) -> anyhow::Result<(u64, u64)> {
    let mut count = 0u64;
    let mut bytes = 0u64;

    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_seconds))
        .unwrap_or(std::time::UNIX_EPOCH);

    let Ok(mut entries) = tokio::fs::read_dir(quarantine_dir).await else {
        return Ok((0, 0));
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Ok(meta) = tokio::fs::metadata(&path).await else { continue };
        if !meta.is_file() { continue }
        if meta.modified().unwrap_or(std::time::UNIX_EPOCH) > cutoff { continue }
        let size = meta.len();
        match tokio::fs::remove_file(&path).await {
            Ok(()) => { count += 1; bytes += size; }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => { count += 1; }
            Err(e) => warn!(path = %path.display(), error = %e, "gc: failed to delete quarantined file"),
        }
    }

    Ok((count, bytes))
}
