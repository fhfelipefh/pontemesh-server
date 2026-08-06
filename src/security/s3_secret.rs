use crate::{
    config::PontemeshHome,
    security::{random::secure_url_token, secrets::restrict_secret_file},
};
use anyhow::Context;
use std::fs;

const S3_SECRET_ENCRYPTION_KEY_FILE: &str = "s3SecretEncryptionKey";

pub fn s3_secret_encryption_key(paths: &PontemeshHome) -> anyhow::Result<String> {
    let path = paths.secrets_dir().join(S3_SECRET_ENCRYPTION_KEY_FILE);
    if path.exists() {
        return fs::read_to_string(&path)
            .map(|value| value.trim().to_owned())
            .with_context(|| format!("failed to read {}", path.display()));
    }

    fs::create_dir_all(paths.secrets_dir())
        .with_context(|| format!("failed to create {}", paths.secrets_dir().display()))?;
    let key = secure_url_token("pm_s3_master_", 32);
    fs::write(&path, &key).with_context(|| format!("failed to write {}", path.display()))?;
    restrict_secret_file(&path)?;
    Ok(key)
}
