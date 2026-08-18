use crate::config::GcSection;

#[derive(Debug, Clone)]
pub struct GcConfig {
    pub enabled: bool,
    pub candidate_scan_interval_seconds: u64,
    pub quarantine_period_seconds: u64,
    pub batch_size: usize,
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
            quarantine_period_seconds: section.quarantine_period_seconds,
            batch_size: section.batch_size.clamp(1, 1000),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gc_config_clamping() {
        let mut section = GcSection::default();
        section.candidate_scan_interval_seconds = 2; // below min 10
        section.batch_size = 2000; // above max 1000
        section.full_gc_interval_seconds = 100; // below min 3600
        section.sweep_lease_seconds = 5; // below min 30
        section.temp_file_max_age_seconds = 10; // below min 300

        let config = GcConfig::from(section);
        assert_eq!(config.candidate_scan_interval_seconds, 10);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.full_gc_interval_seconds, 3600);
        assert_eq!(config.sweep_lease_seconds, 30);
        assert_eq!(config.temp_file_max_age_seconds, 300);
    }
}
