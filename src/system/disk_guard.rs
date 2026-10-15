use super::storage::filesystem_usage;
use crate::config::StorageGuardsSection;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiskLevel {
    Ok,
    Warning,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskGuardStatus {
    pub enabled: bool,
    pub level: DiskLevel,
    pub used_percent: Option<f64>,
    pub available_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub warning_percent: f64,
    pub degraded_percent: f64,
    pub block_percent: f64,
}

pub fn check(path: &Path, guards: &StorageGuardsSection) -> DiskGuardStatus {
    if !guards.enabled {
        return DiskGuardStatus {
            enabled: false,
            level: DiskLevel::Ok,
            used_percent: None,
            available_bytes: None,
            total_bytes: None,
            warning_percent: guards.warning_percent,
            degraded_percent: guards.degraded_percent,
            block_percent: guards.block_percent,
        };
    }

    let (total_bytes, available_bytes, used_percent) = match filesystem_usage(path) {
        Some((total, available)) => {
            let used = if total == 0 {
                0.0
            } else {
                ((total.saturating_sub(available)) as f64 / total as f64) * 100.0
            };
            (Some(total), Some(available), Some(used))
        }
        None => (None, None, None),
    };

    let level = match used_percent {
        None => DiskLevel::Ok,
        Some(pct) if pct >= guards.block_percent => DiskLevel::Blocked,
        Some(pct) if pct >= guards.degraded_percent => DiskLevel::Degraded,
        Some(pct) if pct >= guards.warning_percent => DiskLevel::Warning,
        _ => DiskLevel::Ok,
    };

    DiskGuardStatus {
        enabled: true,
        level,
        used_percent,
        available_bytes,
        total_bytes,
        warning_percent: guards.warning_percent,
        degraded_percent: guards.degraded_percent,
        block_percent: guards.block_percent,
    }
}

pub fn enforce(path: &Path, guards: &StorageGuardsSection) -> anyhow::Result<()> {
    let status = check(path, guards);
    if status.level == DiskLevel::Blocked {
        let pct = status.used_percent.unwrap_or(0.0);
        anyhow::bail!(
            "storage capacity exceeded: disk is {pct:.1}% full (block threshold: {:.1}%). \
             Free up space or raise the block_percent threshold in config.",
            guards.block_percent
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(std::path::PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("pontemesh-disk-guard-{name}-{nanos}"));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_check_when_disabled() {
        let guards = StorageGuardsSection {
            enabled: false,
            warning_percent: 80.0,
            degraded_percent: 90.0,
            block_percent: 95.0,
        };
        let status = check(Path::new("/dummy/path"), &guards);

        assert!(!status.enabled);
        assert_eq!(status.level, DiskLevel::Ok);
        assert!(status.used_percent.is_none());
        assert!(status.available_bytes.is_none());
        assert!(status.total_bytes.is_none());
        assert_eq!(status.warning_percent, 80.0);
        assert_eq!(status.degraded_percent, 90.0);
        assert_eq!(status.block_percent, 95.0);
    }

    #[test]
    fn test_check_level_thresholds_with_live_path() {
        let dir = TestDir::new("thresholds");
        let path = dir.path();

        let ok_guards = StorageGuardsSection {
            enabled: true,
            warning_percent: 101.0,
            degraded_percent: 102.0,
            block_percent: 103.0,
        };
        let status_ok = check(path, &ok_guards);
        assert!(status_ok.enabled);
        if status_ok.total_bytes.is_some() {
            assert_eq!(status_ok.level, DiskLevel::Ok);
            assert!(status_ok.used_percent.is_some());
            let pct = status_ok.used_percent.unwrap();
            assert!((0.0..=100.0).contains(&pct));

            let warning_guards = StorageGuardsSection {
                enabled: true,
                warning_percent: 0.0,
                degraded_percent: 101.0,
                block_percent: 102.0,
            };
            let status_warning = check(path, &warning_guards);
            assert_eq!(status_warning.level, DiskLevel::Warning);

            let degraded_guards = StorageGuardsSection {
                enabled: true,
                warning_percent: 0.0,
                degraded_percent: 0.0,
                block_percent: 101.0,
            };
            let status_degraded = check(path, &degraded_guards);
            assert_eq!(status_degraded.level, DiskLevel::Degraded);

            let block_guards = StorageGuardsSection {
                enabled: true,
                warning_percent: 0.0,
                degraded_percent: 0.0,
                block_percent: 0.0,
            };
            let status_blocked = check(path, &block_guards);
            assert_eq!(status_blocked.level, DiskLevel::Blocked);
        }
    }

    #[test]
    fn test_enforce_returns_ok_or_err() {
        let dir = TestDir::new("enforce");
        let path = dir.path();

        let ok_guards = StorageGuardsSection {
            enabled: true,
            warning_percent: 80.0,
            degraded_percent: 90.0,
            block_percent: 101.0,
        };
        assert!(enforce(path, &ok_guards).is_ok());

        let disabled_guards = StorageGuardsSection {
            enabled: false,
            warning_percent: 0.0,
            degraded_percent: 0.0,
            block_percent: 0.0,
        };
        assert!(enforce(path, &disabled_guards).is_ok());

        let block_guards = StorageGuardsSection {
            enabled: true,
            warning_percent: 0.0,
            degraded_percent: 0.0,
            block_percent: 0.0,
        };
        if filesystem_usage(path).is_some() {
            let res = enforce(path, &block_guards);
            assert!(res.is_err());
            let err_msg = res.unwrap_err().to_string();
            assert!(err_msg.contains("storage capacity exceeded"));
        }
    }
}
