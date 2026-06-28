use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeEnvironment {
    Native,
    Container,
    Unknown,
}

pub fn detect_environment() -> RuntimeEnvironment {
    if Path::new("/.dockerenv").exists() {
        return RuntimeEnvironment::Container;
    }

    match fs::read_to_string("/proc/1/cgroup") {
        Ok(content)
            if content.contains("docker")
                || content.contains("kubepods")
                || content.contains("containerd") =>
        {
            RuntimeEnvironment::Container
        }
        Ok(_) => RuntimeEnvironment::Native,
        Err(_) => RuntimeEnvironment::Unknown,
    }
}

pub fn is_container() -> bool {
    matches!(detect_environment(), RuntimeEnvironment::Container)
}
