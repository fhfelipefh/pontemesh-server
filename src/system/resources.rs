use crate::system::environment;
use serde::Serialize;
use std::{fs, thread, time::Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    pub cpu_usage_percent: Option<f64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_usage_percent: Option<f64>,
    pub process_memory_bytes: Option<u64>,
    pub source: String,
    pub warnings: Vec<String>,
}

pub fn collect() -> ResourceUsage {
    if environment::is_container() {
        return collect_cgroup();
    }
    collect_procfs()
}

fn collect_cgroup() -> ResourceUsage {
    let mut warnings = Vec::new();
    let memory_used = read_u64("/sys/fs/cgroup/memory.current");
    let memory_total = read_cgroup_memory_limit();
    let process_memory = read_process_rss_bytes();

    if memory_used.is_none() {
        warnings.push("Container memory usage could not be collected from cgroups.".to_owned());
    }
    if memory_total.is_none() {
        warnings.push("Container memory limit could not be collected from cgroups.".to_owned());
    }

    let cpu_usage_percent = match read_cgroup_cpu_usage_percent() {
        Some(value) => Some(value),
        None => {
            warnings.push("CPU usage could not be collected from cgroups.".to_owned());
            None
        }
    };

    ResourceUsage {
        cpu_usage_percent,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        memory_usage_percent: percent(memory_used, memory_total),
        process_memory_bytes: process_memory,
        source: "cgroup".to_owned(),
        warnings,
    }
}

fn collect_procfs() -> ResourceUsage {
    let mut warnings = Vec::new();
    let memory_total = read_meminfo_bytes("MemTotal:");
    let memory_available = read_meminfo_bytes("MemAvailable:");
    let memory_used = match (memory_total, memory_available) {
        (Some(total), Some(available)) => Some(total.saturating_sub(available)),
        _ => {
            warnings
                .push("System memory usage could not be collected from /proc/meminfo.".to_owned());
            None
        }
    };
    let process_memory = read_process_rss_bytes();

    let cpu_usage_percent = match read_procfs_cpu_usage_percent() {
        Some(value) => Some(value),
        None => {
            warnings.push("CPU usage could not be collected from /proc.".to_owned());
            None
        }
    };

    ResourceUsage {
        cpu_usage_percent,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        memory_usage_percent: percent(memory_used, memory_total),
        process_memory_bytes: process_memory,
        source: if warnings.len() >= 3 {
            "unavailable".to_owned()
        } else {
            "sysinfo".to_owned()
        },
        warnings,
    }
}

fn percent(used: Option<u64>, total: Option<u64>) -> Option<f64> {
    match (used, total) {
        (Some(used), Some(total)) if total > 0 => Some((used as f64 / total as f64) * 100.0),
        _ => None,
    }
}

fn read_cgroup_cpu_usage_percent() -> Option<f64> {
    let start_usage = read_cgroup_cpu_usage_usec()?;
    let start = std::time::Instant::now();
    thread::sleep(Duration::from_millis(200));
    let elapsed = start.elapsed().as_micros() as f64;
    let end_usage = read_cgroup_cpu_usage_usec()?;
    let cpu_capacity = read_cgroup_cpu_capacity().unwrap_or_else(available_parallelism);
    if elapsed <= 0.0 || cpu_capacity <= 0.0 {
        return None;
    }
    Some(((end_usage.saturating_sub(start_usage)) as f64 / (elapsed * cpu_capacity)) * 100.0)
}

fn read_cgroup_cpu_usage_usec() -> Option<u64> {
    let content = fs::read_to_string("/sys/fs/cgroup/cpu.stat").ok()?;
    content.lines().find_map(|line| {
        let (key, value) = line.split_once(' ')?;
        (key == "usage_usec").then(|| value.parse().ok()).flatten()
    })
}

fn read_cgroup_cpu_capacity() -> Option<f64> {
    let content = fs::read_to_string("/sys/fs/cgroup/cpu.max").ok()?;
    let mut parts = content.split_whitespace();
    let quota = parts.next()?;
    let period: f64 = parts.next()?.parse().ok()?;
    if quota == "max" {
        return Some(available_parallelism());
    }
    let quota: f64 = quota.parse().ok()?;
    (period > 0.0).then_some(quota / period)
}

fn read_cgroup_memory_limit() -> Option<u64> {
    let raw = fs::read_to_string("/sys/fs/cgroup/memory.max").ok()?;
    let trimmed = raw.trim();
    if trimmed == "max" {
        return read_meminfo_bytes("MemTotal:");
    }
    trimmed.parse().ok()
}

fn read_procfs_cpu_usage_percent() -> Option<f64> {
    let (start_proc, start_total) = (read_process_cpu_ticks()?, read_total_cpu_ticks()?);
    thread::sleep(Duration::from_millis(200));
    let (end_proc, end_total) = (read_process_cpu_ticks()?, read_total_cpu_ticks()?);
    let proc_delta = end_proc.saturating_sub(start_proc) as f64;
    let total_delta = end_total.saturating_sub(start_total) as f64;
    if total_delta <= 0.0 {
        return None;
    }
    Some((proc_delta / total_delta) * available_parallelism() * 100.0)
}

fn read_process_cpu_ticks() -> Option<u64> {
    let content = fs::read_to_string("/proc/self/stat").ok()?;
    let end_comm = content.rfind(") ")?;
    let fields: Vec<&str> = content[end_comm + 2..].split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

fn read_total_cpu_ticks() -> Option<u64> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = content.lines().next()?;
    cpu_line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .try_fold(0_u64, |acc, value| value.ok().map(|value| acc + value))
}

fn read_process_rss_bytes() -> Option<u64> {
    read_status_value_kb("VmRSS:").map(|kb| kb * 1024)
}

fn read_status_value_kb(key: &str) -> Option<u64> {
    let content = fs::read_to_string("/proc/self/status").ok()?;
    content.lines().find_map(|line| {
        if !line.starts_with(key) {
            return None;
        }
        line.split_whitespace().nth(1)?.parse::<u64>().ok()
    })
}

fn read_meminfo_bytes(key: &str) -> Option<u64> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    content.lines().find_map(|line| {
        if !line.starts_with(key) {
            return None;
        }
        line.split_whitespace()
            .nth(1)?
            .parse::<u64>()
            .ok()
            .map(|kb| kb * 1024)
    })
}

fn read_u64(path: &str) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn available_parallelism() -> f64 {
    std::thread::available_parallelism()
        .map(|value| value.get() as f64)
        .unwrap_or(1.0)
}
