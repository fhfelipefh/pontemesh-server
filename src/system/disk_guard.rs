use super::storage::filesystem_usage;
use crate::config::StorageGuardsSection;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiskLevel {
    Ok,
    Warning,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
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
