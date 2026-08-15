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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_and_failure_helpers_do_not_panic() {
        event("TEST_EVENT", Some("admin"), "success", "detail info");
        event("TEST_EVENT_NO_USER", None, "success", "detail info");
        failure("TEST_FAILURE", Some("user1"), "invalid credentials");
        failure("TEST_FAILURE_NO_USER", None, "access denied");
    }
}
