mod admin;
mod audit;
mod auth;
mod catalog;
mod config;
mod http;
mod mesh;
mod origin;
mod replica;
mod s3_auth;
mod security;
mod setup;
mod system;
mod web_assets;

use anyhow::{Context, bail};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let paths = config::PontemeshHome::from_env()?;
    paths.ensure_layout()?;
    let internal_secrets = security::secrets::load_or_create_internal_secrets(&paths)?;
    let _ = (
        internal_secrets.instance_secret.len(),
        internal_secrets.session_secret.len(),
        internal_secrets.token_secret.len(),
    );
    let catalog = catalog::Catalog::initialize().await?;
    initialize_s3_bootstrap_key(&paths, &catalog).await?;
    let setup_state = setup::first_run::initialize(&paths)?;

    let web_bind_addr = config::load_http_bind_addr(&paths)?;
    let s3_bind_addr = config::default_s3_bind_addr();
    let web_app = http::web_router(paths.clone(), setup_state.clone(), catalog.clone());
    let s3_app = http::s3_router(paths, setup_state, catalog);
    let web_listener = tokio::net::TcpListener::bind(web_bind_addr)
        .await
        .with_context(|| format!("failed to bind web server at {web_bind_addr}"))?;
    let s3_listener = tokio::net::TcpListener::bind(s3_bind_addr)
        .await
        .with_context(|| format!("failed to bind S3-compatible server at {s3_bind_addr}"))?;

    info!(%web_bind_addr, "pontemesh-server web listener started");
    info!(%s3_bind_addr, "pontemesh-server S3-compatible listener started");

    tokio::try_join!(
        axum::serve(web_listener, web_app).with_graceful_shutdown(shutdown_signal()),
        axum::serve(s3_listener, s3_app).with_graceful_shutdown(shutdown_signal()),
    )
    .context("server listener failed")?;

    Ok(())
}

async fn initialize_s3_bootstrap_key(
    paths: &config::PontemeshHome,
    catalog: &catalog::Catalog,
) -> anyhow::Result<()> {
    let access_key_id = optional_env("PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID");
    let secret = optional_env("PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY");
    let (Some(access_key_id), Some(secret)) = (access_key_id, secret) else {
        if optional_env("PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID").is_some()
            || optional_env("PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY").is_some()
        {
            bail!(
                "PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID and PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY must be set together"
            );
        }
        return Ok(());
    };
    if access_key_id.trim().is_empty() {
        bail!("S3 bootstrap access key id cannot be empty");
    }
    if !access_key_id.starts_with("PMK") && !access_key_id.starts_with("PM") {
        bail!("S3 bootstrap access key id must start with PMK or PM");
    }
    if secret.trim().len() < 20 {
        bail!("S3 bootstrap secret access key must have at least 20 characters");
    }
    let secret_encryption_key = security::s3_secret::s3_secret_encryption_key(paths)?;
    catalog
        .ensure_s3_access_key(
            None,
            Some("bootstrap"),
            &access_key_id,
            &secret,
            &secret_encryption_key,
        )
        .await?;
    Ok(())
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "pontemesh_server=info,tower_http=info".into());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
