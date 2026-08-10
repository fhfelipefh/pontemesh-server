mod admin;
mod audit;
mod auth;
mod catalog;
mod config;
mod gc;
mod health;
mod http;
mod mcp;
mod mesh;
mod origin;
mod replica;
mod replica_runtime;
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
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "setup-agent") {
        let options = setup::agent::SetupAgentOptions::parse(&args[1..])?;
        return setup::agent::run(paths, options).await;
    }

    let internal_secrets = security::secrets::load_or_create_internal_secrets(&paths)?;
    let _ = (
        internal_secrets.instance_secret.len(),
        internal_secrets.session_secret.len(),
        internal_secrets.token_secret.len(),
    );
    let setup_state = setup::first_run::initialize(&paths)?;

    if setup_state.is_required(&paths) {
        let catalog = catalog::Catalog::initialize().await?;
        initialize_s3_bootstrap_key(&paths, &catalog).await?;
        return run_servers(paths, setup_state, catalog).await;
    }

    let instance_config = config::load_instance_config(&paths)?;
    let catalog = catalog::Catalog::initialize().await?;

    match instance_config.instance.role {
        config::InstanceRole::Origin => {
            initialize_s3_bootstrap_key(&paths, &catalog).await?;
            run_servers(paths, setup_state, catalog).await
        }
        config::InstanceRole::ReplicaEdge => {
            let replica_config = config::load_replica_runtime_config(&paths)?;
            tokio::select! {
                result = run_servers(paths, setup_state, catalog) => result,
                result = replica_runtime::run(replica_config) => result,
            }
        }
    }
}

async fn run_servers(
    paths: config::PontemeshHome,
    setup_state: setup::SetupState,
    catalog: catalog::Catalog,
) -> anyhow::Result<()> {
    let web_bind_addr = config::load_http_bind_addr(&paths)?;
    let s3_bind_addr = config::load_s3_bind_addr(&paths)?;
    let web_app = http::web_router(paths.clone(), setup_state.clone(), catalog.clone());
    let s3_app = http::s3_router(paths.clone(), setup_state, catalog.clone());
    let web_listener = tokio::net::TcpListener::bind(web_bind_addr)
        .await
        .with_context(|| format!("failed to bind web server at {web_bind_addr}"))?;
    let s3_listener = tokio::net::TcpListener::bind(s3_bind_addr)
        .await
        .with_context(|| format!("failed to bind S3-compatible server at {s3_bind_addr}"))?;

    info!(%web_bind_addr, "pontemesh-server web listener started");
    info!(%s3_bind_addr, "pontemesh-server S3-compatible listener started");

    let gc_config = config::load_instance_config(&paths)
        .map(|c| gc::config::GcConfig::from(c.gc))
        .unwrap_or_default();
    let gc_metrics = gc::metrics::new_shared();
    let gc_runtime = gc::scheduler::GcRuntime::new(&catalog, paths, gc_config, gc_metrics);

    tokio::try_join!(
        axum::serve(web_listener, web_app).with_graceful_shutdown(shutdown_signal()),
        axum::serve(s3_listener, s3_app).with_graceful_shutdown(shutdown_signal()),
        async { gc_runtime.run().await; Ok(()) },
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
