use crate::{
    catalog,
    config::{self, PontemeshHome},
    http::AppState,
    mcp,
    security::{
        random::secure_url_token, s3_secret::s3_secret_encryption_key,
        secrets::load_or_create_internal_secrets,
    },
    setup::{self, routes::CompleteSetupRequest},
};
use anyhow::{Context, bail};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::Instant,
};

#[derive(Debug)]
pub struct SetupAgentOptions {
    instance_name: String,
    admin_username: String,
    admin_password: Option<String>,
    storage_path: Option<String>,
    http_port: u16,
    mcp_token_name: String,
    mcp_scopes: Vec<String>,
    allow_remote_mcp: bool,
    connection_file: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupAgentReport {
    setup_completed_now: bool,
    setup_already_completed: bool,
    pontemesh_home: String,
    web_url: String,
    s3_endpoint_url: String,
    mcp: SetupAgentMcpReport,
    generated_admin: Option<GeneratedAdminReport>,
    initial_s3_access_key: Option<catalog::CreatedS3AccessKey>,
    connection_file: String,
    safety: SetupAgentSafetyReport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupAgentMcpReport {
    endpoint: String,
    url: String,
    token_name: String,
    token_scopes: Vec<String>,
    token_secret: String,
    connection_config: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedAdminReport {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupAgentSafetyReport {
    localhost_only: bool,
    require_auth: bool,
    secret_disclosure: &'static str,
    data_plane_access: &'static str,
}

impl SetupAgentOptions {
    pub fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut options = Self {
            instance_name: "Ponte Mesh".to_owned(),
            admin_username: "admin".to_owned(),
            admin_password: None,
            storage_path: None,
            http_port: 8080,
            mcp_token_name: "setup-agent".to_owned(),
            mcp_scopes: vec!["read".to_owned(), "write".to_owned(), "admin".to_owned()],
            allow_remote_mcp: false,
            connection_file: None,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--instance-name" => options.instance_name = take_value(args, &mut index)?,
                "--admin-username" => options.admin_username = take_value(args, &mut index)?,
                "--admin-password" => options.admin_password = Some(take_value(args, &mut index)?),
                "--storage-path" => options.storage_path = Some(take_value(args, &mut index)?),
                "--http-port" => options.http_port = take_value(args, &mut index)?.parse()?,
                "--mcp-token-name" => options.mcp_token_name = take_value(args, &mut index)?,
                "--mcp-scopes" => {
                    options.mcp_scopes = parse_scopes(&take_value(args, &mut index)?)?
                }
                "--allow-remote-mcp" => options.allow_remote_mcp = true,
                "--connection-file" => {
                    options.connection_file = Some(PathBuf::from(take_value(args, &mut index)?))
                }
                "--help" | "-h" => bail!("{}", help_text()),
                unknown => bail!("unknown setup-agent argument: {unknown}\n\n{}", help_text()),
            }
            index += 1;
        }

        if options.instance_name.trim().is_empty() {
            bail!("--instance-name cannot be empty");
        }
        if options.admin_username.trim().is_empty() {
            bail!("--admin-username cannot be empty");
        }
        if options.mcp_token_name.trim().is_empty() {
            bail!("--mcp-token-name cannot be empty");
        }
        Ok(options)
    }
}

pub async fn run(paths: PontemeshHome, options: SetupAgentOptions) -> anyhow::Result<()> {
    let internal_secrets = load_or_create_internal_secrets(&paths)?;
    let _ = (
        internal_secrets.instance_secret.len(),
        internal_secrets.session_secret.len(),
        internal_secrets.token_secret.len(),
    );
    let setup_state = setup::SetupState::new();
    let catalog = catalog::Catalog::initialize().await?;
    let state = AppState {
        paths: paths.clone(),
        setup: setup_state.clone(),
        catalog: catalog.clone(),
        started_at: Instant::now(),
    };

    let setup_required = setup_state.is_required(&paths);
    let generated_password = options
        .admin_password
        .clone()
        .unwrap_or_else(|| secure_url_token("pm_admin_", 24));

    let initial_s3_access_key = if setup_required {
        Some(
            super::routes::complete_setup(
                &state,
                CompleteSetupRequest {
                    instance_name: options.instance_name.clone(),
                    role: "origin".to_owned(),
                    admin_username: options.admin_username.clone(),
                    admin_password: generated_password.clone(),
                    http_port: Some(options.http_port),
                    s3_port: None,
                    public_web_url: None,
                    public_s3_url: None,
                    internal_storage_path: options.storage_path.clone(),
                    origin_base_url: None,
                    replica_id: None,
                    replica_token: None,
                    replica_public_endpoint: None,
                    sync_interval_seconds: None,
                    health_interval_seconds: None,
                },
            )
            .await?,
        )
        .flatten()
    } else {
        None
    };

    let settings = catalog
        .update_mcp_settings(catalog::McpSettingsUpdate {
            enabled: true,
            endpoint_path: mcp::config::DEFAULT_ENDPOINT_PATH.to_owned(),
            bind_host: None,
            require_auth: true,
            read_tools_enabled: true,
            write_tools_enabled: options
                .mcp_scopes
                .iter()
                .any(|scope| scope == "write" || scope == "admin"),
            admin_tools_enabled: options.mcp_scopes.iter().any(|scope| scope == "admin"),
            expose_resources: true,
            expose_prompts: true,
            allow_localhost_only: !options.allow_remote_mcp,
        })
        .await?;

    let created_mcp_token = catalog
        .create_mcp_access_token(&options.mcp_token_name, &options.mcp_scopes, None)
        .await?;

    catalog
        .record_audit_event(
            "setup_agent_mcp_enabled",
            Some("setup-agent"),
            "success",
            &format!(
                "localhost_only={}; scopes={}",
                settings.allow_localhost_only,
                options.mcp_scopes.join(",")
            ),
        )
        .await?;

    let web_port = config::load_http_bind_addr(&paths)?.port();
    let s3_port = config::load_s3_bind_addr(&paths)?.port();
    let web_url = format!("http://127.0.0.1:{web_port}");
    let mcp_url = format!("{web_url}{}", settings.endpoint_path);
    let connection_config = mcp_connection_config(&mcp_url, &created_mcp_token.secret);
    let connection_file = options
        .connection_file
        .clone()
        .unwrap_or_else(|| paths.secrets_dir().join("setup-agent-mcp.json"));
    write_secret_json(&connection_file, &connection_config)?;

    let report = SetupAgentReport {
        setup_completed_now: setup_required,
        setup_already_completed: !setup_required,
        pontemesh_home: paths.root().display().to_string(),
        web_url,
        s3_endpoint_url: format!("http://127.0.0.1:{s3_port}"),
        mcp: SetupAgentMcpReport {
            endpoint: settings.endpoint_path,
            url: mcp_url,
            token_name: created_mcp_token.token.name,
            token_scopes: created_mcp_token.token.scopes,
            token_secret: created_mcp_token.secret,
            connection_config,
        },
        generated_admin: (setup_required && options.admin_password.is_none()).then_some(
            GeneratedAdminReport {
                username: options.admin_username,
                password: generated_password,
            },
        ),
        initial_s3_access_key,
        connection_file: connection_file.display().to_string(),
        safety: SetupAgentSafetyReport {
            localhost_only: settings.allow_localhost_only,
            require_auth: settings.require_auth,
            secret_disclosure: "new secrets are shown once and stored only in the generated local connection file",
            data_plane_access: "MCP remains administrative; object traffic stays on S3-compatible and Ponte Mesh endpoints",
        },
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub async fn create_s3_key_for_mcp(
    state: &AppState,
    name: Option<&str>,
) -> anyhow::Result<catalog::CreatedS3AccessKey> {
    let admin = state
        .catalog
        .first_active_admin_user()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no active admin user available for S3 key ownership"))?;
    let secret_encryption_key = s3_secret_encryption_key(&state.paths)?;
    let created = state
        .catalog
        .create_s3_access_key(&admin.id, name, &secret_encryption_key)
        .await?;
    state
        .catalog
        .record_audit_event(
            "s3_access_key_created",
            Some("mcp"),
            "success",
            &format!(
                "access_key_id={}; owner={}",
                created.access_key_id, admin.username
            ),
        )
        .await?;
    Ok(created)
}

fn take_value(args: &[String], index: &mut usize) -> anyhow::Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing value for {}", args[*index - 1]))
}

fn parse_scopes(value: &str) -> anyhow::Result<Vec<String>> {
    let scopes = value
        .split(',')
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if scopes.is_empty() {
        bail!("--mcp-scopes must include at least one scope");
    }
    for scope in &scopes {
        if !matches!(scope.as_str(), "read" | "write" | "admin") {
            bail!("unsupported MCP scope: {scope}");
        }
    }
    Ok(scopes)
}

fn mcp_connection_config(url: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "pontemesh-server",
        "transport": "streamable-http",
        "url": url,
        "method": "POST",
        "headers": {
            "Authorization": format!("Bearer {token}"),
            "Content-Type": "application/json"
        },
        "env": {
            "PONTEMESH_MCP_URL": url,
            "PONTEMESH_MCP_TOKEN": token
        },
        "initialize": {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "pontemesh-setup-agent",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }
    })
}

fn write_secret_json(path: &PathBuf, value: &serde_json::Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn help_text() -> &'static str {
    "Usage: pontemesh-server setup-agent [--instance-name NAME] [--admin-username USER] [--admin-password PASSWORD] [--storage-path PATH] [--http-port PORT] [--mcp-token-name NAME] [--mcp-scopes read,write,admin] [--allow-remote-mcp] [--connection-file PATH]"
}
