use crate::catalog::Catalog;
use crate::config::PontemeshHome;
use crate::gc::candidate::{claim_batch, reclaim_expired_leases};
use crate::gc::config::GcConfig;
use crate::gc::metrics::{GcStatus, SharedMetrics};
use crate::gc::sweeper::{purge_quarantine, sweep_candidate};
use crate::gc::temp_cleaner::clean_stale_temp_files;
use crate::{config, system};
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct GcRuntime {
    pool: PgPool,
    pub paths: PontemeshHome,
    pub config: GcConfig,
    pub metrics: SharedMetrics,
    pub status: Arc<Mutex<GcStatus>>,
}

impl GcRuntime {
    pub fn new(catalog: &Catalog, paths: PontemeshHome, config: GcConfig, metrics: SharedMetrics) -> Self {
        let pool = catalog.db_pool().clone();
        let status = Arc::new(Mutex::new(GcStatus {
            enabled: config.enabled,
            state: "IDLE",
            epoch: 0,
            pending_candidates: 0,
            pending_bytes: 0,
            quarantined_bytes: 0,
            last_full_gc: None,
            last_reclaimed_bytes: 0,
            cycles_total: 0,
            errors_total: 0,
        }));
        Self { pool, paths, config, metrics, status }
    }

    pub fn current_status(&self) -> GcStatus {
        self.status.lock().unwrap().clone()
    }

    pub async fn run(self) {
        if !self.config.enabled {
            info!("gc: disabled by configuration");
            return;
        }

        let scan_interval = Duration::from_secs(self.config.candidate_scan_interval_seconds);
        let mut full_gc_due = tokio::time::Instant::now();

        loop {
            let storage_dir = match config::configured_storage_dir(&self.paths) {
                Ok(path) => path,
                Err(error) => {
                    error!(error = %error, "gc: failed to resolve storage dir, skipping cycle");
                    sleep(scan_interval).await;
                    continue;
                }
            };

            let quarantine_dir = storage_dir.join(".gc").join("quarantine");
            if let Err(error) = tokio::fs::create_dir_all(&quarantine_dir).await {
                warn!(error = %error, "gc: failed to create quarantine dir");
            }

            if let Err(error) = reclaim_expired_leases(&self.pool).await {
                warn!(error = %error, "gc: failed to reclaim expired leases");
            }

            self.run_candidate_sweep(&storage_dir, &quarantine_dir).await;

            if tokio::time::Instant::now() >= full_gc_due {
                self.run_temp_cleanup(&storage_dir).await;
                self.run_quarantine_purge(&quarantine_dir).await;
                full_gc_due = tokio::time::Instant::now()
                    + Duration::from_secs(self.config.full_gc_interval_seconds);
                if let Ok(mut status) = self.status.lock() {
                    status.last_full_gc = Some(chrono::Utc::now().to_rfc3339());
                }
            }

            sleep(scan_interval).await;
        }
    }

    async fn run_candidate_sweep(&self, storage_dir: &PathBuf, quarantine_dir: &PathBuf) {
        let batch_size = i64::try_from(self.config.batch_size).unwrap_or(100);
        let candidates = match claim_batch(
            &self.pool,
            batch_size,
            self.config.sweep_lease_seconds,
        )
        .await
        {
            Ok(c) => c,
            Err(error) => {
                warn!(error = %error, "gc: failed to claim candidate batch");
                self.metrics.record_error();
                return;
            }
        };

        if candidates.is_empty() {
            return;
        }

        info!(count = candidates.len(), "gc: sweeping candidate batch");
        self.metrics.record_cycle_start();

        let mut reclaimed_objects = 0i64;
        let mut reclaimed_bytes = 0i64;

        for candidate in &candidates {
            match sweep_candidate(
                &self.pool,
                candidate,
                storage_dir,
                quarantine_dir,
                self.config.max_retries,
            )
            .await
            {
                Ok(Some(bytes)) => {
                    reclaimed_objects += 1;
                    reclaimed_bytes += i64::try_from(bytes).unwrap_or(0);
                }
                Ok(None) => {}
                Err(error) => {
                    warn!(id = %candidate.id, error = %error, "gc: sweep error");
                    self.metrics.record_error();
                    if let Err(err) = crate::gc::candidate::mark_failed(
                        &self.pool,
                        &candidate.id,
                        self.config.max_retries,
                        &error.to_string(),
                    )
                    .await
                    {
                        warn!(error = %err, "gc: failed to record candidate failure");
                    }
                }
            }
        }

        if reclaimed_objects > 0 {
            self.metrics.record_reclaimed(reclaimed_objects, reclaimed_bytes);
            info!(objects = reclaimed_objects, bytes = reclaimed_bytes, "gc: sweep batch completed");
        }

        if let Ok(mut status) = self.status.lock() {
            status.last_reclaimed_bytes = self.metrics.bytes_reclaimed_total.load(Ordering::Relaxed);
            status.cycles_total = self.metrics.cycles_total.load(Ordering::Relaxed);
            status.errors_total = self.metrics.errors_total.load(Ordering::Relaxed);
        }
    }

    async fn run_temp_cleanup(&self, storage_dir: &PathBuf) {
        match clean_stale_temp_files(storage_dir, self.config.temp_file_max_age_seconds).await {
            Ok((count, bytes)) if count > 0 => info!(count, bytes, "gc: stale temp files removed"),
            Err(error) => { warn!(error = %error, "gc: temp file cleanup failed"); self.metrics.record_error(); }
            _ => {}
        }
    }

    async fn run_quarantine_purge(&self, quarantine_dir: &PathBuf) {
        match purge_quarantine(quarantine_dir, self.config.quarantine_period_seconds).await {
            Ok((count, bytes)) if count > 0 => info!(count, bytes, "gc: quarantined files permanently deleted"),
            Err(error) => { warn!(error = %error, "gc: quarantine purge failed"); self.metrics.record_error(); }
            _ => {}
        }
    }
}

pub async fn trigger_dry_run(
    pool: &PgPool,
    storage_dir: &std::path::Path,
) -> anyhow::Result<crate::gc::admin::DryRunResult> {
    let (pending_candidates, pending_bytes) =
        crate::gc::candidate::pending_stats(pool).await?;
    let available_bytes = system::storage::filesystem_usage(storage_dir)
        .map(|(_, a)| a)
        .unwrap_or(0);
    Ok(crate::gc::admin::DryRunResult {
        pending_candidates,
        pending_bytes,
        available_bytes,
        would_reclaim_bytes: pending_bytes,
    })
}
