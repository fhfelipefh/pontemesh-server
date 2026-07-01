use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

pub const DEFAULT_PONTEMESH_HOME: &str = "/var/pontemesh_home";
pub const PONTEMESH_STORAGE_PATH_ENV: &str = "PONTEMESH_STORAGE_PATH";
pub const PONTEMESH_DATABASE_URL_ENV: &str = "PONTEMESH_DATABASE_URL";
const PONTEMESH_HTTP_HOST_ENV: &str = "PONTEMESH_HTTP_HOST";
const PONTEMESH_HTTP_PORT_ENV: &str = "PONTEMESH_HTTP_PORT";
const PONTEMESH_WEB_PORT_ENV: &str = "PONTEMESH_WEB_PORT";
const PONTEMESH_S3_PORT_ENV: &str = "PONTEMESH_S3_PORT";
const DEFAULT_HTTP_PORT: u16 = 8080;
const DEFAULT_S3_PORT: u16 = 9000;

#[derive(Debug, Clone)]
pub struct PontemeshHome {
    root: PathBuf,
}

impl PontemeshHome {
    pub fn from_path(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            bail!("PONTEMESH_HOME cannot be empty");
        }

        Ok(Self { root })
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let root = env::var_os("PONTEMESH_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_PONTEMESH_HOME));

        Self::from_path(root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("config.toml")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn storage_dir(&self) -> PathBuf {
        self.data_dir().join("storage")
    }

    pub fn storage_dir_from_env(&self) -> anyhow::Result<Option<PathBuf>> {
        let Some(path) = env::var_os(PONTEMESH_STORAGE_PATH_ENV).map(PathBuf::from) else {
            return Ok(None);
        };

        if path.as_os_str().is_empty() {
            bail!("{PONTEMESH_STORAGE_PATH_ENV} cannot be empty");
        }

        Ok(Some(path))
    }

    pub fn effective_storage_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(self
            .storage_dir_from_env()?
            .unwrap_or_else(|| self.storage_dir()))
    }

    pub fn secrets_dir(&self) -> PathBuf {
        self.root.join("secrets")
    }

    pub fn initial_admin_token_file(&self) -> PathBuf {
        self.secrets_dir().join("initialAdminToken")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    pub fn setup_lock_file(&self) -> PathBuf {
        self.state_dir().join("setup.lock")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn ensure_layout(&self) -> anyhow::Result<()> {
        for dir in [
            self.config_dir(),
            self.effective_storage_dir()?,
            self.secrets_dir(),
            self.state_dir(),
            self.logs_dir(),
        ] {
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub instance: InstanceSection,
    pub http: HttpSection,
    pub storage: StorageSection,
    pub replica: Option<ReplicaSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSection {
    pub name: String,
    pub role: InstanceRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstanceRole {
    Origin,
    ReplicaEdge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSection {
    pub bind: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    pub local: LocalStorageSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageSection {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaSection {
    pub origin_base_url: String,
    pub replica_id: String,
    pub replica_token: String,
    pub public_endpoint: String,
    pub sync_interval_seconds: Option<u64>,
    pub health_interval_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ReplicaRuntimeConfig {
    pub origin_base_url: String,
    pub replica_id: String,
    pub replica_token: String,
    pub public_endpoint: String,
    pub sync_interval_seconds: u64,
    pub health_interval_seconds: u64,
    pub storage_path: PathBuf,
}

pub fn load_http_bind_addr(paths: &PontemeshHome) -> anyhow::Result<SocketAddr> {
    if !paths.setup_lock_file().exists() || !paths.config_file().exists() {
        return Ok(default_bind_addr());
    }

    let config = load_instance_config(paths)?;

    let ip: IpAddr = config
        .http
        .bind
        .parse()
        .with_context(|| format!("invalid HTTP bind address '{}'", config.http.bind))?;

    Ok(SocketAddr::new(ip, config.http.port))
}

pub fn default_bind_addr() -> SocketAddr {
    default_web_bind_addr()
}

pub fn default_web_bind_addr() -> SocketAddr {
    let host = env::var(PONTEMESH_HTTP_HOST_ENV)
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let port = env::var(PONTEMESH_WEB_PORT_ENV)
        .or_else(|_| env::var(PONTEMESH_HTTP_PORT_ENV))
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_HTTP_PORT);
    SocketAddr::new(host, port)
}

pub fn default_s3_bind_addr() -> SocketAddr {
    let host = env::var(PONTEMESH_HTTP_HOST_ENV)
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let port = env::var(PONTEMESH_S3_PORT_ENV)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_S3_PORT);
    SocketAddr::new(host, port)
}

pub fn load_instance_config(paths: &PontemeshHome) -> anyhow::Result<InstanceConfig> {
    let raw_config = fs::read_to_string(paths.config_file())
        .with_context(|| format!("failed to read {}", paths.config_file().display()))?;
    toml::from_str(&raw_config).context("failed to parse Ponte Mesh config.toml")
}

pub fn configured_instance_role(paths: &PontemeshHome) -> anyhow::Result<Option<InstanceRole>> {
    if !paths.setup_lock_file().exists() || !paths.config_file().exists() {
        return Ok(None);
    }

    Ok(Some(load_instance_config(paths)?.instance.role))
}

pub fn require_instance_role(paths: &PontemeshHome, expected: InstanceRole) -> anyhow::Result<()> {
    let Some(role) = configured_instance_role(paths)? else {
        bail!("initial setup must be completed before this operation");
    };

    if role != expected {
        bail!(
            "operation requires instance role {}; current role is {}",
            expected.as_config_value(),
            role.as_config_value()
        );
    }

    Ok(())
}

impl InstanceRole {
    pub fn as_config_value(self) -> &'static str {
        match self {
            InstanceRole::Origin => "origin",
            InstanceRole::ReplicaEdge => "replica-edge",
        }
    }
}

pub fn configured_storage_dir(paths: &PontemeshHome) -> anyhow::Result<PathBuf> {
    if let Some(path) = paths.storage_dir_from_env()? {
        return Ok(path);
    }

    if paths.config_file().exists() {
        return Ok(load_instance_config(paths)?.storage.local.path);
    }

    Ok(paths.storage_dir())
}

pub fn load_replica_runtime_config(paths: &PontemeshHome) -> anyhow::Result<ReplicaRuntimeConfig> {
    let config = load_instance_config(paths)?;
    if !matches!(config.instance.role, InstanceRole::ReplicaEdge) {
        bail!("replica runtime requires instance.role = replica-edge");
    }
    let replica = config
        .replica
        .ok_or_else(|| anyhow::anyhow!("replica-edge config requires [replica] section"))?;
    let storage_path = configured_storage_dir(paths)?;
    validate_url(&replica.origin_base_url, "replica.origin_base_url")?;
    validate_url(&replica.public_endpoint, "replica.public_endpoint")?;
    validate_non_empty(&replica.replica_id, "replica.replica_id")?;
    validate_non_empty(&replica.replica_token, "replica.replica_token")?;
    Ok(ReplicaRuntimeConfig {
        origin_base_url: replica.origin_base_url.trim_end_matches('/').to_owned(),
        replica_id: replica.replica_id,
        replica_token: replica.replica_token,
        public_endpoint: replica.public_endpoint.trim_end_matches('/').to_owned(),
        sync_interval_seconds: replica.sync_interval_seconds.unwrap_or(30).max(5),
        health_interval_seconds: replica.health_interval_seconds.unwrap_or(30).max(5),
        storage_path,
    })
}

pub fn database_url_from_env() -> anyhow::Result<String> {
    let url = env::var(PONTEMESH_DATABASE_URL_ENV)
        .with_context(|| format!("{PONTEMESH_DATABASE_URL_ENV} must be set to a PostgreSQL URL"))?;
    if url.trim().is_empty() {
        bail!("{PONTEMESH_DATABASE_URL_ENV} cannot be empty");
    }
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        bail!("{PONTEMESH_DATABASE_URL_ENV} must use postgres:// or postgresql://");
    }
    Ok(url)
}

fn validate_non_empty(value: &str, field: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(())
}

fn validate_url(value: &str, field: &str) -> anyhow::Result<()> {
    validate_non_empty(value, field)?;
    if !value.starts_with("http://") && !value.starts_with("https://") {
        bail!("{field} must be an HTTP or HTTPS URL");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_replica_runtime_config() {
        let home = test_home("replica-config");
        home.ensure_layout().expect("layout");
        fs::write(
            home.setup_lock_file(),
            "completed_at = \"2026-06-30T00:00:00Z\"\n",
        )
        .expect("setup lock");
        fs::write(
            home.config_file(),
            r#"
[instance]
name = "edge"
role = "replica-edge"

[http]
bind = "127.0.0.1"
port = 8080

[storage.local]
path = "/tmp/pontemesh-edge-storage"

[replica]
origin_base_url = "https://origin.example.com/"
replica_id = "replica-1"
replica_token = "replica-token"
public_endpoint = "https://edge.example.com/"
sync_interval_seconds = 1
health_interval_seconds = 2
"#,
        )
        .expect("config");

        let config = load_replica_runtime_config(&home).expect("replica config");
        assert_eq!(config.origin_base_url, "https://origin.example.com");
        assert_eq!(config.public_endpoint, "https://edge.example.com");
        assert_eq!(config.sync_interval_seconds, 5);
        assert_eq!(config.health_interval_seconds, 5);
    }

    #[test]
    fn replica_runtime_config_requires_replica_section() {
        let home = test_home("replica-config-missing");
        home.ensure_layout().expect("layout");
        fs::write(
            home.config_file(),
            r#"
[instance]
name = "edge"
role = "replica-edge"

[http]
bind = "127.0.0.1"
port = 8080

[storage.local]
path = "/tmp/pontemesh-edge-storage"
"#,
        )
        .expect("config");

        let error = load_replica_runtime_config(&home).expect_err("missing replica section");
        assert!(error.to_string().contains("[replica]"));
    }

    #[test]
    fn cargo_features_do_not_select_instance_role() {
        let cargo_toml = include_str!("../../Cargo.toml");
        assert!(!cargo_toml.contains("origin = []"));
        assert!(!cargo_toml.contains("replica = []"));
        assert!(!cargo_toml.contains("replication = []"));
        for role in ["origin", "replica", "replication"] {
            assert!(!cargo_toml.contains(&format!("--features {role}")));
        }
    }

    fn test_home(name: &str) -> PontemeshHome {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        PontemeshHome::from_path(std::env::temp_dir().join(format!("pontemesh-{name}-{nanos}")))
            .expect("test home")
    }
}
