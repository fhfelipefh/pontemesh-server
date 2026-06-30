use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

const MAX_APPLICATION_LOGS: usize = 200;

static APPLICATION_LOGS: OnceLock<Mutex<VecDeque<ApplicationLogEntry>>> = OnceLock::new();

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: &'static str,
    pub target: &'static str,
    pub message: String,
}

pub fn info(target: &'static str, message: impl Into<String>) {
    record("info", target, message);
}

pub fn error(target: &'static str, message: impl Into<String>) {
    record("error", target, message);
}

pub fn recent(limit: usize) -> Vec<ApplicationLogEntry> {
    let logs = logs().lock().expect("application log lock poisoned");
    let take = limit.clamp(1, MAX_APPLICATION_LOGS);
    logs.iter().rev().take(take).cloned().collect()
}

fn record(level: &'static str, target: &'static str, message: impl Into<String>) {
    let entry = ApplicationLogEntry {
        timestamp: Utc::now(),
        level,
        target,
        message: message.into(),
    };

    let mut logs = logs().lock().expect("application log lock poisoned");
    if logs.len() >= MAX_APPLICATION_LOGS {
        logs.pop_front();
    }
    logs.push_back(entry);
}

fn logs() -> &'static Mutex<VecDeque<ApplicationLogEntry>> {
    APPLICATION_LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_APPLICATION_LOGS)))
}
