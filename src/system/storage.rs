use anyhow::{Context, bail};
use serde::Serialize;
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

fn filesystem_usage(path: &Path) -> Option<(u64, u64)> {
    let canonical = path.canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|disk| canonical.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| (disk.total_space(), disk.available_space()))
}
