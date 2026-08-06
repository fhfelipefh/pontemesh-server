use crate::{config::PontemeshHome, security::random::secure_url_token};
use anyhow::Context;
use std::{
    fs::{self, OpenOptions},
    io::Write,
};
use subtle::ConstantTimeEq;

const INITIAL_TOKEN_PREFIX: &str = "pm_init_";
const INITIAL_TOKEN_RANDOM_BYTES: usize = 32;

pub fn read_or_create_initial_admin_token(paths: &PontemeshHome) -> anyhow::Result<String> {
    let token_path = paths.initial_admin_token_file();
    if token_path.exists() {
        return read_initial_admin_token(paths);
    }

    let token = secure_url_token(INITIAL_TOKEN_PREFIX, INITIAL_TOKEN_RANDOM_BYTES);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(&token_path)
        .with_context(|| format!("failed to create {}", token_path.display()))?;
    file.write_all(token.as_bytes())
        .with_context(|| format!("failed to write {}", token_path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finalize {}", token_path.display()))?;

    Ok(token)
}

pub fn read_initial_admin_token(paths: &PontemeshHome) -> anyhow::Result<String> {
    let token_path = paths.initial_admin_token_file();
    Ok(fs::read_to_string(&token_path)
        .with_context(|| format!("failed to read {}", token_path.display()))?
        .trim()
        .to_owned())
}

pub fn initial_admin_token_matches(paths: &PontemeshHome, candidate: &str) -> anyhow::Result<bool> {
    let expected = read_initial_admin_token(paths)?;
    Ok(expected.as_bytes().ct_eq(candidate.as_bytes()).into())
}

pub fn invalidate_initial_admin_token(paths: &PontemeshHome) -> anyhow::Result<()> {
    let token_path = paths.initial_admin_token_file();
    if token_path.exists() {
        fs::remove_file(&token_path)
            .with_context(|| format!("failed to remove {}", token_path.display()))?;
    }

    Ok(())
}
