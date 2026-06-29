use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

pub const DEFAULT_PONTEMESH_HOME: &str = "/var/pontemesh_home";
pub const PONTEMESH_STORAGE_PATH_ENV: &str = "PONTEMESH_STORAGE_PATH";
const PONTEMESH_HTTP_HOST_ENV: &str = "PONTEMESH_HTTP_HOST";
const PONTEMESH_HTTP_PORT_ENV: &str = "PONTEMESH_HTTP_PORT";
const DEFAULT_HTTP_PORT: u16 = 8080;

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

    pub fn catalog_database_file(&self) -> PathBuf {
        self.data_dir().join("catalog.sqlite")
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
    pub admin: AdminSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSection {
    pub name: String,
    pub role: InstanceRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct AdminSection {
    pub username: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
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
    let host = env::var(PONTEMESH_HTTP_HOST_ENV)
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let port = env::var(PONTEMESH_HTTP_PORT_ENV)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_HTTP_PORT);
    SocketAddr::new(host, port)
}

pub fn load_instance_config(paths: &PontemeshHome) -> anyhow::Result<InstanceConfig> {
    let raw_config = fs::read_to_string(paths.config_file())
        .with_context(|| format!("failed to read {}", paths.config_file().display()))?;
    toml::from_str(&raw_config).context("failed to parse Ponte Mesh config.toml")
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
