pub mod agent;
pub mod first_run;
pub mod routes;
pub mod token;

use crate::config::PontemeshHome;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct SetupState {
    inner: Arc<SetupStateInner>,
}

#[derive(Debug)]
struct SetupStateInner {
    unlock_sessions: Mutex<HashSet<String>>,
}

impl SetupState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SetupStateInner {
                unlock_sessions: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub fn is_required(&self, paths: &PontemeshHome) -> bool {
        !paths.setup_lock_file().exists()
    }

    pub fn add_unlock_session(&self, session: String) {
        let mut sessions = self
            .inner
            .unlock_sessions
            .lock()
            .expect("poisoned setup session lock");
        sessions.insert(session);
    }

    pub fn is_unlocked(&self, session: Option<&str>) -> bool {
        let Some(session) = session else {
            return false;
        };

        let sessions = self
            .inner
            .unlock_sessions
            .lock()
            .expect("poisoned setup session lock");
        sessions.contains(session)
    }

    pub fn clear_unlock_sessions(&self) {
        let mut sessions = self
            .inner
            .unlock_sessions
            .lock()
            .expect("poisoned setup session lock");
        sessions.clear();
    }
}
