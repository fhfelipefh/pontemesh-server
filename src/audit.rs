use tracing::{info, warn};

pub fn event(event: &str, username: Option<&str>, outcome: &str, detail: &str) {
    info!(
        target: "pontemesh_audit",
        event,
        username = username.unwrap_or(""),
        outcome,
        detail
    );
}

pub fn failure(event: &str, username: Option<&str>, detail: &str) {
    warn!(
        target: "pontemesh_audit",
        event,
        username = username.unwrap_or(""),
        outcome = "failure",
        detail
    );
}
