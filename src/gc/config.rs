use crate::config::GcSection;

#[derive(Debug, Clone)]
pub struct GcConfig {
    pub enabled: bool,
    pub candidate_scan_interval_seconds: u64,
    pub grace_period_seconds: u64,
    pub quarantine_period_seconds: u64,
    pub batch_size: usize,
    pub max_concurrent_io: usize,
    pub full_gc_interval_seconds: u64,
    pub sweep_lease_seconds: u64,
    pub max_retries: u32,
    pub temp_file_max_age_seconds: u64,
}

impl From<GcSection> for GcConfig {
    fn from(section: GcSection) -> Self {
        Self {
            enabled: section.enabled,
            candidate_scan_interval_seconds: section.candidate_scan_interval_seconds.max(10),
            grace_period_seconds: section.grace_period_seconds,
            quarantine_period_seconds: section.quarantine_period_seconds,
            batch_size: section.batch_size.clamp(1, 1000),
            max_concurrent_io: section.max_concurrent_io.clamp(1, 32),
            full_gc_interval_seconds: section.full_gc_interval_seconds.max(3600),
            sweep_lease_seconds: section.sweep_lease_seconds.max(30),
            max_retries: section.max_retries,
            temp_file_max_age_seconds: section.temp_file_max_age_seconds.max(300),
        }
    }
}

impl Default for GcConfig {
    fn default() -> Self {
        Self::from(GcSection::default())
    }
}
