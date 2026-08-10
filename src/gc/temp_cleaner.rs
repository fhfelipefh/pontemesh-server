use std::path::Path;
use tracing::{info, warn};

pub async fn clean_stale_temp_files(storage_dir: &Path, max_age_seconds: u64) -> anyhow::Result<(u64, u64)> {
    let mut count = 0u64;
    let mut bytes = 0u64;

    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_seconds))
        .unwrap_or(std::time::UNIX_EPOCH);

    let patterns: &[&str] = &[".tmp", ".part"];
    let dirs_to_scan = [
        storage_dir.join("tmp").join("uploads"),
        storage_dir.to_owned(),
    ];

    for dir in &dirs_to_scan {
        if !dir.exists() {
            continue;
        }
        let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !patterns.iter().any(|p| name.ends_with(p)) {
                continue;
            }
            let Ok(meta) = tokio::fs::metadata(&path).await else {
                continue;
            };
            let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            if modified > cutoff {
                continue;
            }
            let size = meta.len();
            match tokio::fs::remove_file(&path).await {
                Ok(()) => {
                    count += 1;
                    bytes += size;
                    info!(path = %path.display(), bytes = size, "gc: stale temp file removed");
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    warn!(path = %path.display(), error = %error, "gc: failed to remove stale temp file");
                }
            }
        }
    }

    Ok((count, bytes))
}
