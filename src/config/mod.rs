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
const PONTEMESH_PUBLIC_WEB_URL_ENV: &str = "PONTEMESH_PUBLIC_WEB_URL";
const PONTEMESH_PUBLIC_S3_URL_ENV: &str = "PONTEMESH_PUBLIC_S3_URL";
const PONTEMESH_PUBLIC_S3_ENDPOINT_ENV: &str = "PONTEMESH_PUBLIC_S3_ENDPOINT";
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
    #[serde(default = "default_s3_section")]
    pub s3: S3Section,
    #[serde(default, rename = "public")]
    pub public_endpoints: PublicEndpointsSection,
    pub storage: StorageSection,
    pub replica: Option<ReplicaSection>,
    #[serde(default, rename = "garbage_collector")]
    pub gc: GcSection,
    #[serde(default)]
    pub webhook: WebhookSection,
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
pub struct S3Section {
    pub bind: String,
    pub port: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublicEndpointsSection {
    pub web_url: Option<String>,
    pub s3_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    pub local: LocalStorageSection,
    #[serde(default)]
    pub guards: StorageGuardsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default = "default_webhook_cron")]
    pub cron: String,
}

impl Default for WebhookSection {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            cron: default_webhook_cron(),
        }
    }
}

fn default_webhook_cron() -> String {
    "*/15 * * * *".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageGuardsSection {
    #[serde(default = "default_guards_enabled")]
    pub enabled: bool,
    #[serde(default = "default_warning_percent")]
    pub warning_percent: f64,
    #[serde(default = "default_degraded_percent")]
    pub degraded_percent: f64,
    #[serde(default = "default_block_percent")]
    pub block_percent: f64,
}

impl Default for StorageGuardsSection {
    fn default() -> Self {
        Self {
            enabled: default_guards_enabled(),
            warning_percent: default_warning_percent(),
            degraded_percent: default_degraded_percent(),
            block_percent: default_block_percent(),
        }
    }
}

fn default_guards_enabled() -> bool {
    true
}

fn default_warning_percent() -> f64 {
    80.0
}

fn default_degraded_percent() -> f64 {
    90.0
}

fn default_block_percent() -> f64 {
    95.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcSection {
    #[serde(default = "default_gc_enabled")]
    pub enabled: bool,
    #[serde(default = "default_candidate_scan_interval")]
    pub candidate_scan_interval_seconds: u64,
    #[serde(default = "default_grace_period")]
    pub grace_period_seconds: u64,
    #[serde(default = "default_quarantine_period")]
    pub quarantine_period_seconds: u64,
    #[serde(default = "default_gc_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_max_concurrent_io")]
    pub max_concurrent_io: usize,
    #[serde(default = "default_full_gc_interval")]
    pub full_gc_interval_seconds: u64,
    #[serde(default = "default_sweep_lease")]
    pub sweep_lease_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_temp_file_max_age")]
    pub temp_file_max_age_seconds: u64,
}

impl Default for GcSection {
    fn default() -> Self {
        Self {
            enabled: default_gc_enabled(),
            candidate_scan_interval_seconds: default_candidate_scan_interval(),
            grace_period_seconds: default_grace_period(),
            quarantine_period_seconds: default_quarantine_period(),
            batch_size: default_gc_batch_size(),
            max_concurrent_io: default_max_concurrent_io(),
            full_gc_interval_seconds: default_full_gc_interval(),
            sweep_lease_seconds: default_sweep_lease(),
            max_retries: default_max_retries(),
            temp_file_max_age_seconds: default_temp_file_max_age(),
        }
    }
}

fn default_gc_enabled() -> bool {
    true
}
fn default_candidate_scan_interval() -> u64 {
    60
}
fn default_grace_period() -> u64 {
    7200
}
fn default_quarantine_period() -> u64 {
    86400
}
fn default_gc_batch_size() -> usize {
    100
}
fn default_max_concurrent_io() -> usize {
    4
}
fn default_full_gc_interval() -> u64 {
    86400
}
fn default_sweep_lease() -> u64 {
    300
}
fn default_max_retries() -> u32 {
    10
}
fn default_temp_file_max_age() -> u64 {
    3600
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
    pub storage_guards: StorageGuardsSection,
}

pub fn load_http_bind_addr(paths: &PontemeshHome) -> anyhow::Result<SocketAddr> {
    if !paths.setup_lock_file().exists() || !paths.config_file().exists() {
        return Ok(default_bind_addr());
    }

    let config = load_instance_config(paths)?;

    let configured_ip: IpAddr = config
        .http
        .bind
        .parse()
        .with_context(|| format!("invalid HTTP bind address '{}'", config.http.bind))?;

    let ip = environment_ip(PONTEMESH_HTTP_HOST_ENV)?.unwrap_or(configured_ip);
    let port = environment_port(&[PONTEMESH_WEB_PORT_ENV, PONTEMESH_HTTP_PORT_ENV])?
        .unwrap_or(config.http.port);

    Ok(SocketAddr::new(ip, port))
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

pub fn load_s3_bind_addr(paths: &PontemeshHome) -> anyhow::Result<SocketAddr> {
    if !paths.setup_lock_file().exists() || !paths.config_file().exists() {
        return Ok(default_s3_bind_addr());
    }

    let config = load_instance_config(paths)?;
    let configured_ip: IpAddr = config
        .s3
        .bind
        .parse()
        .with_context(|| format!("invalid S3 bind address '{}'", config.s3.bind))?;
    let ip = environment_ip(PONTEMESH_HTTP_HOST_ENV)?.unwrap_or(configured_ip);
    let port = environment_port(&[PONTEMESH_S3_PORT_ENV])?.unwrap_or(config.s3.port);

    Ok(SocketAddr::new(ip, port))
}

pub fn configured_public_web_url(paths: &PontemeshHome) -> anyhow::Result<Option<String>> {
    configured_public_url(paths, &[PONTEMESH_PUBLIC_WEB_URL_ENV], |config| {
        config.public_endpoints.web_url
    })
}

pub fn configured_public_s3_url(paths: &PontemeshHome) -> anyhow::Result<Option<String>> {
    configured_public_url(
        paths,
        &[
            PONTEMESH_PUBLIC_S3_URL_ENV,
            PONTEMESH_PUBLIC_S3_ENDPOINT_ENV,
        ],
        |config| config.public_endpoints.s3_url,
    )
}

fn configured_public_url(
    paths: &PontemeshHome,
    environment_names: &[&str],
    from_config: impl FnOnce(InstanceConfig) -> Option<String>,
) -> anyhow::Result<Option<String>> {
    for name in environment_names {
        if let Ok(value) = env::var(name) {
            if value.trim().is_empty() {
                continue;
            }
            return normalize_public_url(Some(value), name);
        }
    }

    if !paths.config_file().exists() {
        return Ok(None);
    }

    normalize_public_url(from_config(load_instance_config(paths)?), "public endpoint")
}

fn normalize_public_url(value: Option<String>, field: &str) -> anyhow::Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Ok(None);
    }
    validate_url(value, field)?;
    Ok(Some(value.to_owned()))
}

fn environment_ip(name: &str) -> anyhow::Result<Option<IpAddr>> {
    let Ok(value) = env::var(name) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(value.parse().with_context(|| {
        format!("{name} must be a valid IP address")
    })?))
}

fn environment_port(names: &[&str]) -> anyhow::Result<Option<u16>> {
    for name in names {
        if let Ok(value) = env::var(name) {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            return Ok(Some(
                value
                    .parse()
                    .with_context(|| format!("{name} must be a valid TCP port"))?,
            ));
        }
    }
    Ok(None)
}

fn default_s3_section() -> S3Section {
    S3Section {
        bind: Ipv4Addr::UNSPECIFIED.to_string(),
        port: DEFAULT_S3_PORT,
    }
}

pub fn load_instance_config(paths: &PontemeshHome) -> anyhow::Result<InstanceConfig> {
    let raw_config = fs::read_to_string(paths.config_file())
        .with_context(|| format!("failed to read {}", paths.config_file().display()))?;
    toml::from_str(&raw_config).context("failed to parse Ponte Mesh config.toml")
}

pub fn update_instance_name(paths: &PontemeshHome, name: &str) -> anyhow::Result<InstanceConfig> {
    let name = validate_instance_name(name)?;
    let mut config = load_instance_config(paths)?;
    config.instance.name = name;
    let serialized = toml::to_string_pretty(&config).context("failed to serialize config.toml")?;
    fs::write(paths.config_file(), serialized)
        .with_context(|| format!("failed to write {}", paths.config_file().display()))?;
    Ok(config)
}

pub fn update_storage_guards(
    paths: &PontemeshHome,
    guards: StorageGuardsSection,
) -> anyhow::Result<InstanceConfig> {
    validate_storage_guards(&guards)?;
    let mut config = load_instance_config(paths)?;
    config.storage.guards = guards;
    let serialized = toml::to_string_pretty(&config).context("failed to serialize config.toml")?;
    fs::write(paths.config_file(), serialized)
        .with_context(|| format!("failed to write {}", paths.config_file().display()))?;
    Ok(config)
}

pub fn update_webhook(
    paths: &PontemeshHome,
    webhook: WebhookSection,
) -> anyhow::Result<InstanceConfig> {
    let mut config = load_instance_config(paths)?;
    config.webhook = webhook;
    let serialized = toml::to_string_pretty(&config).context("failed to serialize config.toml")?;
    fs::write(paths.config_file(), serialized)
        .with_context(|| format!("failed to write {}", paths.config_file().display()))?;
    Ok(config)
}

pub fn validate_storage_guards(guards: &StorageGuardsSection) -> anyhow::Result<()> {
    let values = [
        ("warning_percent", guards.warning_percent),
        ("degraded_percent", guards.degraded_percent),
        ("block_percent", guards.block_percent),
    ];
    for (name, value) in values {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            bail!("{name} must be between 0 and 100");
        }
    }
    if guards.warning_percent >= guards.degraded_percent
        || guards.degraded_percent >= guards.block_percent
    {
        bail!("storage thresholds must satisfy warning_percent < degraded_percent < block_percent");
    }
    Ok(())
}

pub fn validate_instance_name(name: &str) -> anyhow::Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("instance name cannot be empty");
    }
    if name.chars().count() > 100 {
        bail!("instance name cannot exceed 100 characters");
    }
    if name.chars().any(char::is_control) {
        bail!("instance name cannot contain control characters");
    }
    Ok(name.to_owned())
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
        storage_guards: config.storage.guards,
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
    fn validates_instance_names() {
        assert_eq!(
            validate_instance_name("  Ponte Mesh  ").unwrap(),
            "Ponte Mesh"
        );
        assert!(validate_instance_name("   ").is_err());
        assert!(validate_instance_name("bad\nname").is_err());
        assert!(validate_instance_name(&"x".repeat(101)).is_err());
    }

    #[test]
    fn validates_storage_guard_threshold_order() {
        assert!(validate_storage_guards(&StorageGuardsSection::default()).is_ok());
        assert!(
            validate_storage_guards(&StorageGuardsSection {
                enabled: true,
                warning_percent: 90.0,
                degraded_percent: 80.0,
                block_percent: 95.0,
            })
            .is_err()
        );
        assert!(
            validate_storage_guards(&StorageGuardsSection {
                enabled: true,
                warning_percent: 80.0,
                degraded_percent: 90.0,
                block_percent: 101.0,
            })
            .is_err()
        );
    }

    #[test]
    fn persists_storage_guard_thresholds() {
        let home = test_home("storage-guards-update");
        home.ensure_layout().expect("layout");
        fs::write(
            home.config_file(),
            r#"
[instance]
name = "origin"
role = "origin"

[http]
bind = "127.0.0.1"
port = 8080

[storage.local]
path = "/tmp/pontemesh-origin-storage"
"#,
        )
        .expect("config");

        update_storage_guards(
            &home,
            StorageGuardsSection {
                enabled: true,
                warning_percent: 82.0,
                degraded_percent: 91.0,
                block_percent: 97.0,
            },
        )
        .expect("updated guards");

        let persisted = load_instance_config(&home).expect("persisted config");
        assert_eq!(persisted.storage.guards.warning_percent, 82.0);
        assert_eq!(persisted.storage.guards.degraded_percent, 91.0);
        assert_eq!(persisted.storage.guards.block_percent, 97.0);
    }

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

        let persisted = load_instance_config(&home).expect("persisted config");
        assert_eq!(persisted.s3.bind, "0.0.0.0");
        assert_eq!(persisted.s3.port, 9000);
        assert!(persisted.public_endpoints.web_url.is_none());
        assert!(persisted.public_endpoints.s3_url.is_none());

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

    #[test]
    fn public_urls_are_normalized_and_validated() {
        assert_eq!(
            normalize_public_url(
                Some(" https://origin.example.com:9443/ ".to_owned()),
                "public endpoint"
            )
            .expect("valid endpoint"),
            Some("https://origin.example.com:9443".to_owned())
        );
        assert!(
            normalize_public_url(
                Some("ftp://origin.example.com".to_owned()),
                "public endpoint"
            )
            .is_err()
        );
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
