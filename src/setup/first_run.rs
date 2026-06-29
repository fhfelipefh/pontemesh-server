use crate::{config::PontemeshHome, setup::SetupState};
use anyhow::Context;
use std::{fs, path::Path};
use tracing::warn;

use super::token::read_or_create_initial_admin_token;

pub fn initialize(paths: &PontemeshHome) -> anyhow::Result<SetupState> {
    paths.ensure_layout()?;
    warn_if_ephemeral_container_home(paths.root());

    let setup_required = !paths.setup_lock_file().exists();
    if setup_required {
        let token = read_or_create_initial_admin_token(paths)?;
        log_initial_setup_message(paths, &token);
    }

    Ok(SetupState::new())
}

fn log_initial_setup_message(paths: &PontemeshHome, token: &str) {
    let token_path = paths.initial_admin_token_file();
    let banner = format!(
        "\n*************************************************************\n\
         *************************************************************\n\
         *************************************************************\n\n\
         Ponte Mesh initial setup is required.\n\n\
         An initial admin token has been generated.\n\
         Please use the following token to unlock Ponte Mesh:\n\n\
         {token}\n\n\
         This token is also available at:\n\n\
         {token_path}\n\n\
         If running with Docker, you can also read it with:\n\n\
         docker compose -p ponte-mesh -f docker/docker-compose.yml exec server cat {token_path}\n\n\
         *************************************************************\n\
         *************************************************************\n\
         *************************************************************",
        token_path = token_path.display()
    );

    warn!("{banner}");
}

fn warn_if_ephemeral_container_home(home: &Path) {
    if !is_probably_container() {
        return;
    }

    match is_mount_point(home) {
        Ok(true) => {}
        Ok(false) => warn!(
            "WARNING: no persistent volume detected for PONTEMESH_HOME. If this container is removed, configuration, secrets and objects may be lost."
        ),
        Err(error) => warn!(
            %error,
            "could not determine whether PONTEMESH_HOME is backed by a persistent volume"
        ),
    }
}

fn is_probably_container() -> bool {
    if Path::new("/.dockerenv").exists() {
        return true;
    }

    fs::read_to_string("/proc/1/cgroup")
        .map(|content| {
            content.contains("docker")
                || content.contains("kubepods")
                || content.contains("containerd")
        })
        .unwrap_or(false)
}

fn is_mount_point(path: &Path) -> anyhow::Result<bool> {
    let canonical_home = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")
        .context("failed to read /proc/self/mountinfo")?;

    Ok(mountinfo.lines().any(|line| {
        line.split_whitespace()
            .nth(4)
            .map(|mount_point| Path::new(mount_point) == canonical_home)
            .unwrap_or(false)
    }))
}
