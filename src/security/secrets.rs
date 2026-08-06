use crate::{config::PontemeshHome, security::random::secure_url_token};
use anyhow::Context;
use std::{fs, path::Path};
use tracing::info;

const INTERNAL_SECRET_FILES: [&str; 3] = ["instance_secret", "session_secret", "token_secret"];

#[derive(Debug, Clone)]
pub struct InternalSecrets {
    pub instance_secret: String,
    pub session_secret: String,
    pub token_secret: String,
}

pub fn load_or_create_internal_secrets(paths: &PontemeshHome) -> anyhow::Result<InternalSecrets> {
    Ok(InternalSecrets {
        instance_secret: load_or_create_secret(paths, INTERNAL_SECRET_FILES[0])?,
        session_secret: load_or_create_secret(paths, INTERNAL_SECRET_FILES[1])?,
        token_secret: load_or_create_secret(paths, INTERNAL_SECRET_FILES[2])?,
    })
}

fn load_or_create_secret(paths: &PontemeshHome, name: &str) -> anyhow::Result<String> {
    let path = paths.secrets_dir().join(name);
    if path.exists() {
        return fs::read_to_string(&path)
            .map(|value| value.trim().to_owned())
            .with_context(|| format!("failed to read {}", path.display()));
    }

    fs::create_dir_all(paths.secrets_dir())
        .with_context(|| format!("failed to create {}", paths.secrets_dir().display()))?;
    let value = secure_url_token("pm_internal_", 48);
    fs::write(&path, &value).with_context(|| format!("failed to write {}", path.display()))?;
    restrict_secret_file(&path)?;
    info!("Generated missing instance secret: {name}");
    Ok(value)
}

#[cfg(unix)]
pub fn restrict_secret_file(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?
        .permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to protect {}", path.display()))
}

#[cfg(not(unix))]
pub fn restrict_secret_file(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn first_load_generates_internal_secrets_and_second_load_reuses_them() {
        let root = std::env::temp_dir().join(format!(
            "pontemesh-secrets-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let paths = PontemeshHome::from_path(&root).expect("test home");
        paths.ensure_layout().expect("layout");

        let first = load_or_create_internal_secrets(&paths).expect("first load");
        let second = load_or_create_internal_secrets(&paths).expect("second load");

        assert_eq!(first.instance_secret, second.instance_secret);
        assert_eq!(first.session_secret, second.session_secret);
        assert_eq!(first.token_secret, second.token_secret);
        for name in INTERNAL_SECRET_FILES {
            let path = paths.secrets_dir().join(name);
            assert!(path.exists(), "{name} should be persisted");
            assert!(
                !fs::read_to_string(path)
                    .expect("secret file")
                    .trim()
                    .is_empty()
            );
        }
    }
}
