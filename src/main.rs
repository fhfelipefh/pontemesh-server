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

use anyhow::Context;
use sha2::{Digest, Sha256};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let paths = config::PontemeshHome::from_env()?;
    paths.ensure_layout()?;
    let catalog = catalog::Catalog::initialize().await?;
    initialize_s3_bootstrap_key(&catalog).await?;
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

async fn initialize_s3_bootstrap_key(catalog: &catalog::Catalog) -> anyhow::Result<()> {
    let Some(access_key_id) = std::env::var("PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID").ok() else {
        return Ok(());
    };
    let secret = std::env::var("PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY").context(
        "PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY must be set when PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID is set",
    )?;
    let secret_key_hash = format!("{:x}", Sha256::digest(secret.as_bytes()));
    catalog
        .ensure_s3_access_key(&access_key_id, &secret_key_hash)
        .await?;
    Ok(())
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
