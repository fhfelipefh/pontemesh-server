use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct GcMetrics {
    pub cycles_total: AtomicU64,
    pub objects_reclaimed_total: AtomicI64,
    pub bytes_reclaimed_total: AtomicI64,
    pub errors_total: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GcStatus {
    pub enabled: bool,
    pub state: &'static str,
    pub epoch: i64,
    pub pending_candidates: i64,
    pub pending_bytes: i64,
    pub quarantined_bytes: i64,
    pub last_full_gc: Option<String>,
    pub last_reclaimed_bytes: i64,
    pub cycles_total: u64,
    pub errors_total: u64,
}

pub type SharedMetrics = Arc<GcMetrics>;

pub fn new_shared() -> SharedMetrics {
    Arc::new(GcMetrics::default())
}

impl GcMetrics {
    pub fn record_cycle_start(&self) {
        self.cycles_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_reclaimed(&self, objects: i64, bytes: i64) {
        self.objects_reclaimed_total
            .fetch_add(objects, Ordering::Relaxed);
        self.bytes_reclaimed_total
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_metrics_record_and_get() {
        let metrics = GcMetrics::default();
        assert_eq!(metrics.cycles_total.load(Ordering::Relaxed), 0);

        metrics.record_cycle_start();
        assert_eq!(metrics.cycles_total.load(Ordering::Relaxed), 1);

        metrics.record_reclaimed(5, 1024);
        assert_eq!(metrics.objects_reclaimed_total.load(Ordering::Relaxed), 5);
        assert_eq!(metrics.bytes_reclaimed_total.load(Ordering::Relaxed), 1024);

        metrics.record_error();
        assert_eq!(metrics.errors_total.load(Ordering::Relaxed), 1);
    }
}
