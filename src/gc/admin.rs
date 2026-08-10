use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunResult {
    pub pending_candidates: i64,
    pub pending_bytes: i64,
    pub available_bytes: u64,
    pub would_reclaim_bytes: i64,
}
