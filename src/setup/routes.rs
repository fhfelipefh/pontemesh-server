use crate::{
    audit,
    config::{
        HttpSection, InstanceConfig, InstanceRole, InstanceSection, LocalStorageSection,
        ReplicaSection, StorageSection,
    },
    http::AppState,
    security::{
        password::hash_admin_password, random::secure_url_token,
        s3_secret::s3_secret_encryption_key,
    },
    setup::token,
};
use anyhow::{Context, bail};
use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

const SETUP_SESSION_COOKIE: &str = "pm_setup_unlock";

#[derive(Debug, Deserialize)]
pub struct UnlockRequest {
    token: String,
}

#[derive(Debug, Serialize)]
pub struct UnlockResponse {
    unlocked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusResponse {
    setup_required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSetupRequest {
    pub instance_name: String,
    pub role: String,
    pub admin_username: String,
    pub admin_password: String,
    pub http_port: Option<u16>,
    #[serde(alias = "storageLocalPath")]
    pub internal_storage_path: Option<String>,
    pub origin_base_url: Option<String>,
    pub replica_id: Option<String>,
    pub replica_token: Option<String>,
    pub replica_public_endpoint: Option<String>,
    pub sync_interval_seconds: Option<u64>,
    pub health_interval_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn status(State(state): State<AppState>) -> Response {
    Json(SetupStatusResponse {
        setup_required: state.setup.is_required(&state.paths),
    })
    .into_response()
}

pub async fn unlock(State(state): State<AppState>, body: Bytes) -> Response {
    if !state.setup.is_required(&state.paths) {
        return setup_not_available();
    }

    let payload: UnlockRequest = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(error) => return bad_request(anyhow::anyhow!("invalid JSON payload: {error}")),
    };

    match token::initial_admin_token_matches(&state.paths, payload.token.trim()) {
        Ok(true) => {
            let session = secure_url_token("", 32);
            state.setup.add_unlock_session(session.clone());

            let mut response =
                (StatusCode::OK, Json(UnlockResponse { unlocked: true })).into_response();
            let cookie =
                format!("{SETUP_SESSION_COOKIE}={session}; Path=/; HttpOnly; SameSite=Strict");
            response.headers_mut().insert(
                header::SET_COOKIE,
                HeaderValue::from_str(&cookie).expect("valid setup cookie"),
            );
            response
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid initial admin token".to_owned(),
            }),
        )
            .into_response(),
        Err(error) => internal_error(error),
    }
}

pub async fn complete(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if !state.setup.is_required(&state.paths) {
        return setup_not_available();
    }

    if !state
        .setup
        .is_unlocked(read_setup_session(&headers).as_deref())
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "setup token has not been unlocked".to_owned(),
            }),
        )
            .into_response();
    }

    let payload: CompleteSetupRequest = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(error) => return bad_request(anyhow::anyhow!("invalid JSON payload: {error}")),
    };

    match complete_setup(&state, payload).await {
        Ok(created_s3_key) => {
            info!("Ponte Mesh initial setup completed");
            (
                StatusCode::OK,
                [(header::SET_COOKIE, clear_setup_cookie())],
                Json(serde_json::json!({
                    "ready": true,
                    "initialS3AccessKey": created_s3_key
                })),
            )
                .into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub(crate) async fn complete_setup(
    state: &AppState,
    payload: CompleteSetupRequest,
) -> anyhow::Result<Option<crate::catalog::CreatedS3AccessKey>> {
    let instance_name = non_empty(payload.instance_name, "instanceName")?;
    let admin_username = non_empty(payload.admin_username, "adminUsername")?;
    let admin_password = non_empty(payload.admin_password, "adminPassword")?;
    if admin_password.len() < 8 {
        bail!("adminPassword must have at least 8 characters");
    }

    let role = match payload.role.as_str() {
        "origin" => InstanceRole::Origin,
        "replica-edge" => InstanceRole::ReplicaEdge,
        _ => bail!("role must be origin or replica-edge"),
    };

    let http_port = payload.http_port.unwrap_or(8080);
    let storage_path = resolve_setup_storage_path(state, payload.internal_storage_path)?;
    validate_storage_path(&storage_path)?;

    let replica = match role {
        InstanceRole::Origin => None,
        InstanceRole::ReplicaEdge => Some(ReplicaSection {
            origin_base_url: http_url(payload.origin_base_url, "originBaseUrl")?
                .trim_end_matches('/')
                .to_owned(),
            replica_id: non_empty_option(payload.replica_id, "replicaId")?,
            replica_token: non_empty_option(payload.replica_token, "replicaToken")?,
            public_endpoint: http_url(payload.replica_public_endpoint, "replicaPublicEndpoint")?
                .trim_end_matches('/')
                .to_owned(),
            sync_interval_seconds: payload.sync_interval_seconds,
            health_interval_seconds: payload.health_interval_seconds,
        }),
    };

    let password_hash = hash_admin_password(&admin_password)?;
    let admin_user_id = state
        .catalog
        .create_initial_admin_user(&admin_username, &password_hash)
        .await?;

    let created_s3_key = if matches!(role, InstanceRole::Origin) {
        let secret_encryption_key = s3_secret_encryption_key(&state.paths)?;
        let created_s3_key = state
            .catalog
            .create_s3_access_key(
                &admin_user_id,
                Some("default-admin-key"),
                &secret_encryption_key,
            )
            .await?;
        audit::event(
            "s3_access_key_created",
            Some(&admin_username),
            "success",
            &format!("access_key_id={}", created_s3_key.access_key_id),
        );
        state
            .catalog
            .record_audit_event(
                "s3_access_key_created",
                Some(&admin_username),
                "success",
                &format!("access_key_id={}", created_s3_key.access_key_id),
            )
            .await?;
        Some(created_s3_key)
    } else {
        None
    };

    let config = InstanceConfig {
        instance: InstanceSection {
            name: instance_name,
            role,
        },
        http: HttpSection {
            bind: "0.0.0.0".to_owned(),
            port: http_port,
        },
        storage: StorageSection {
            local: LocalStorageSection { path: storage_path },
        },
        replica,
    };

    let config_toml = toml::to_string_pretty(&config).context("failed to serialize config.toml")?;
    fs::write(state.paths.config_file(), config_toml)
        .with_context(|| format!("failed to write {}", state.paths.config_file().display()))?;

    token::invalidate_initial_admin_token(&state.paths)?;
    fs::write(
        state.paths.setup_lock_file(),
        format!("completed_at = \"{}\"\n", chrono::Utc::now().to_rfc3339()),
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            state.paths.setup_lock_file().display()
        )
    })?;

    state.setup.clear_unlock_sessions();
    Ok(created_s3_key)
}

fn resolve_setup_storage_path(
    state: &AppState,
    setup_storage_path: Option<String>,
) -> anyhow::Result<PathBuf> {
    if let Some(storage_path) = state.paths.storage_dir_from_env()? {
        return Ok(storage_path);
    }

    if let Some(storage_path) = setup_storage_path {
        let trimmed = storage_path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    Ok(state.paths.storage_dir())
}

fn validate_storage_path(storage_path: &Path) -> anyhow::Result<()> {
    if !storage_path.is_absolute() {
        bail!(
            "storage path must be an absolute internal server path: {}",
            storage_path.display()
        );
    }

    fs::create_dir_all(storage_path).with_context(|| {
        format!(
            "failed to create storage directory {}",
            storage_path.display()
        )
    })?;

    let metadata = fs::metadata(storage_path)
        .with_context(|| format!("failed to inspect storage path {}", storage_path.display()))?;
    if !metadata.is_dir() {
        bail!(
            "storage path is not a directory: {}",
            storage_path.display()
        );
    }

    let probe_path = storage_path.join(format!(
        ".pontemesh-write-test-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let mut probe_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .with_context(|| {
            format!(
                "failed to create temporary storage test file {}",
                probe_path.display()
            )
        })?;

    probe_file
        .write_all(b"pontemesh storage validation\n")
        .with_context(|| {
            format!(
                "failed to write temporary storage test file {}",
                probe_path.display()
            )
        })?;
    probe_file.sync_all().with_context(|| {
        format!(
            "failed to sync temporary storage test file {}",
            probe_path.display()
        )
    })?;
    drop(probe_file);

    fs::remove_file(&probe_path).with_context(|| {
        format!(
            "failed to remove temporary storage test file {}",
            probe_path.display()
        )
    })?;

    Ok(())
}

fn non_empty(value: String, field: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(trimmed.to_owned())
}

fn non_empty_option(value: Option<String>, field: &str) -> anyhow::Result<String> {
    non_empty(
        value.ok_or_else(|| anyhow::anyhow!("{field} is required for replica-edge setup"))?,
        field,
    )
}

fn http_url(value: Option<String>, field: &str) -> anyhow::Result<String> {
    let url = non_empty_option(value, field)?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        bail!("{field} must be an HTTP or HTTPS URL");
    }
    Ok(url)
}

fn read_setup_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SETUP_SESSION_COOKIE).then(|| value.to_owned())
    })
}

fn clear_setup_cookie() -> HeaderValue {
    HeaderValue::from_static("pm_setup_unlock=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict")
}

pub async fn api_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "API endpoint not found".to_owned(),
        }),
    )
        .into_response()
}

fn setup_not_available() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "initial setup is not available".to_owned(),
        }),
    )
        .into_response()
}

fn bad_request(error: anyhow::Error) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn internal_error(error: anyhow::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}
