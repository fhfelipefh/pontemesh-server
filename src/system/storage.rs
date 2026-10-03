use anyhow::{Context, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub path: String,
    pub exists: bool,
    pub writable: bool,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub used_percent: Option<f64>,
    pub warnings: Vec<String>,
}

pub fn status(path: &Path) -> StorageStatus {
    let mut warnings = Vec::new();
    let exists_before = path.exists();
    let writable = match ensure_writable(path) {
        Ok(()) => true,
        Err(error) => {
            warnings.push(error.to_string());
            false
        }
    };

    let (total_bytes, available_bytes, used_bytes, used_percent) = match filesystem_usage(path) {
        Some((total, available)) => {
            let used = total.saturating_sub(available);
            let percent = if total == 0 {
                None
            } else {
                Some((used as f64 / total as f64) * 100.0)
            };
            (Some(total), Some(available), Some(used), percent)
        }
        None => {
            warnings.push("Filesystem usage could not be collected for storage path.".to_owned());
            (None, None, None, None)
        }
    };

    StorageStatus {
        path: path.display().to_string(),
        exists: exists_before || path.exists(),
        writable,
        total_bytes,
        available_bytes,
        used_bytes,
        used_percent,
        warnings,
    }
}

pub fn ensure_writable(path: &Path) -> anyhow::Result<()> {
    if !path.is_absolute() {
        bail!("storage path must be absolute: {}", path.display());
    }

    fs::create_dir_all(path)
        .with_context(|| format!("failed to create storage directory {}", path.display()))?;

    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect storage path {}", path.display()))?;
    if !metadata.is_dir() {
        bail!("storage path is not a directory: {}", path.display());
    }

    let probe_path = path.join(format!(
        ".pontemesh-write-test-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .with_context(|| {
            format!(
                "failed to create temporary storage test file {}",
                probe_path.display()
            )
        })?;
    file.write_all(b"pontemesh storage validation\n")
        .with_context(|| {
            format!(
                "failed to write temporary storage test file {}",
                probe_path.display()
            )
        })?;
    file.sync_all().with_context(|| {
        format!(
            "failed to sync temporary storage test file {}",
            probe_path.display()
        )
    })?;
    drop(file);
    fs::remove_file(&probe_path).with_context(|| {
        format!(
            "failed to remove temporary storage test file {}",
            probe_path.display()
        )
    })?;
    Ok(())
}

pub fn filesystem_usage(path: &Path) -> Option<(u64, u64)> {
    let canonical = path.canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|disk| canonical.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| (disk.total_space(), disk.available_space()))
}

use crate::config::PontemeshHome;
use crate::system::disk_guard::DiskLevel;

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDriveInfo {
    pub id: String,
    pub path: String,
    pub is_primary: bool,
    pub exists: bool,
    pub writable: bool,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub used_percent: Option<f64>,
    pub level: DiskLevel,
    pub warnings: Vec<String>,
}

pub const STRATEGY_MOST_AVAILABLE_FREE_SPACE: &str = "MOST_AVAILABLE_FREE_SPACE";
pub const STRATEGY_ROUND_ROBIN: &str = "ROUND_ROBIN";

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePoolStatus {
    pub allocation_strategy: String,
    pub drives: Vec<StorageDriveInfo>,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub used_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDrainResult {
    pub source_path: String,
    pub target_path: String,
    pub objects_migrated: i64,
    pub bytes_migrated: i64,
}

pub fn pool_status(paths: &PontemeshHome) -> anyhow::Result<StoragePoolStatus> {
    let config = crate::config::load_instance_config(paths)?;
    let guards = &config.storage.guards;
    let configured = crate::config::configured_storage_drives(paths)?;
    let drive_paths: Vec<(PathBuf, bool)> = configured
        .into_iter()
        .map(|path| {
            let is_primary = path == config.storage.local.path;
            (path, is_primary)
        })
        .collect();

    let mut drives = Vec::new();
    let mut agg_total = 0u64;
    let mut agg_available = 0u64;
    let mut has_usage = false;

    for (idx, (drive_path, is_primary)) in drive_paths.iter().enumerate() {
        let drive_status = status(drive_path);
        let guard_check = crate::system::disk_guard::check(drive_path, guards);

        if let (Some(tot), Some(avail)) = (drive_status.total_bytes, drive_status.available_bytes) {
            agg_total = agg_total.saturating_add(tot);
            agg_available = agg_available.saturating_add(avail);
            has_usage = true;
        }

        drives.push(StorageDriveInfo {
            id: format!("drive-{}", idx + 1),
            path: drive_path.display().to_string(),
            is_primary: *is_primary,
            exists: drive_status.exists,
            writable: drive_status.writable,
            total_bytes: drive_status.total_bytes,
            available_bytes: drive_status.available_bytes,
            used_bytes: drive_status.used_bytes,
            used_percent: drive_status.used_percent,
            level: guard_check.level,
            warnings: drive_status.warnings,
        });
    }

    let (total_bytes, available_bytes, used_bytes, used_percent) = if has_usage {
        let used = agg_total.saturating_sub(agg_available);
        let pct = if agg_total == 0 {
            None
        } else {
            Some((used as f64 / agg_total as f64) * 100.0)
        };
        (Some(agg_total), Some(agg_available), Some(used), pct)
    } else {
        (None, None, None, None)
    };

    let strategy = config
        .storage
        .local
        .allocation_strategy
        .unwrap_or_else(|| "MOST_AVAILABLE_FREE_SPACE".to_owned());

    Ok(StoragePoolStatus {
        allocation_strategy: strategy,
        drives,
        total_bytes,
        available_bytes,
        used_bytes,
        used_percent,
    })
}

pub fn select_target_drive(paths: &PontemeshHome) -> anyhow::Result<PathBuf> {
    if !paths.config_file().exists() {
        return crate::config::configured_storage_dir(paths);
    }
    let config = crate::config::load_instance_config(paths)?;
    let mut drives = vec![config.storage.local.path.clone()];
    for extra in &config.storage.local.extra_paths {
        if !drives.contains(extra) {
            drives.push(extra.clone());
        }
    }

    let guards = &config.storage.guards;
    let mut eligible: Vec<(PathBuf, u64)> = Vec::new();

    for drive in &drives {
        let guard_check = crate::system::disk_guard::check(drive, guards);
        if guard_check.level == DiskLevel::Blocked {
            continue;
        }
        if ensure_writable(drive).is_err() {
            continue;
        }
        let available = guard_check.available_bytes.unwrap_or(0);
        eligible.push((drive.clone(), available));
    }

    if eligible.is_empty() {
        if guards.enabled {
            bail!(
                "storage capacity exceeded: all configured storage drives are blocked or unwritable"
            );
        } else if let Some(primary) = drives.first() {
            return Ok(primary.clone());
        } else {
            bail!("no storage drives configured");
        }
    }

    let strategy = config
        .storage
        .local
        .allocation_strategy
        .as_deref()
        .unwrap_or("MOST_AVAILABLE_FREE_SPACE");

    match strategy {
        "ROUND_ROBIN" => {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let idx = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % eligible.len();
            Ok(eligible[idx].0.clone())
        }
        _ => {
            eligible.sort_by(|a, b| b.1.cmp(&a.1));
            Ok(eligible[0].0.clone())
        }
    }
}

pub async fn drain_drive(
    paths: &PontemeshHome,
    catalog: &crate::catalog::Catalog,
    source_path: &Path,
    target_path: Option<&Path>,
    remove_from_config: bool,
) -> anyhow::Result<StorageDrainResult> {
    if !source_path.is_absolute() {
        bail!("source drive path must be absolute");
    }
    let source_prefix = source_path.display().to_string();

    let target_drive = match target_path {
        Some(target) => {
            if !target.is_absolute() {
                bail!("target drive path must be absolute");
            }
            if target == source_path {
                bail!("target drive must be different from source drive");
            }
            ensure_writable(target)?;
            target.to_path_buf()
        }
        None => {
            let candidate = select_target_drive(paths)?;
            if candidate == source_path {
                bail!("no alternate healthy storage drive available to receive drained objects");
            }
            candidate
        }
    };

    let versions = catalog
        .find_versions_by_storage_path_prefix(&source_prefix)
        .await?;

    let mut objects_migrated = 0i64;
    let mut bytes_migrated = 0i64;

    for version in versions {
        let old_file = PathBuf::from(&version.storage_path);
        let dest_bucket_dir = target_drive.join("buckets").join(&version.bucket_name);
        tokio::fs::create_dir_all(&dest_bucket_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create destination directory {}",
                    dest_bucket_dir.display()
                )
            })?;
        let dest_file =
            dest_bucket_dir.join(format!("{}-{}", uuid::Uuid::new_v4(), version.sha256));

        if old_file.exists() {
            tokio::fs::copy(&old_file, &dest_file)
                .await
                .with_context(|| {
                    format!(
                        "failed to copy object data from {} to {}",
                        old_file.display(),
                        dest_file.display()
                    )
                })?;
            let bytes = tokio::fs::read(&dest_file)
                .await
                .with_context(|| format!("failed to read copied file {}", dest_file.display()))?;
            let hash = format!("{:x}", Sha256::digest(&bytes));
            if hash != version.sha256 {
                let _ = tokio::fs::remove_file(&dest_file).await;
                bail!(
                    "integrity check failed during drive drain: hash mismatch for {}",
                    version.object_key
                );
            }
            let _ = tokio::fs::remove_file(&old_file).await;
        }

        let new_storage_path = dest_file.display().to_string();
        catalog
            .update_version_storage_path(&version.version_id, &new_storage_path)
            .await?;

        objects_migrated += 1;
        bytes_migrated += version.size_bytes;
    }

    if remove_from_config {
        let _ = crate::config::remove_storage_drive(paths, source_path);
    }

    Ok(StorageDrainResult {
        source_path: source_path.display().to_string(),
        target_path: target_drive.display().to_string(),
        objects_migrated,
        bytes_migrated,
    })
}
