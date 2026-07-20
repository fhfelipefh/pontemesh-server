use anyhow::{Context, bail};
use postgresql_embedded::{PostgreSQL, SettingsBuilder, V18};
use sqlx::PgPool;
use sqlx_core::query::query;
use std::process::Command;
use std::time::Duration;

const DATABASE_NAME: &str = "pontemesh";
const TEST_NAME: &str = "s3_parity_features_cover_versioning_lifecycle_encryption_lock_policy_notifications_and_checksums";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root_dir = std::env::current_dir().context("resolve repository root")?;
    let install_dir = root_dir.join("target").join("embedded-postgres");
    let run_dir = root_dir
        .join("target")
        .join("embedded-postgres-data")
        .join(format!("s3-parity-{}", std::process::id()));
    let data_dir = run_dir.join("data");
    let password_file = run_dir.join("pgpass");

    std::fs::create_dir_all(&run_dir).context("create embedded PostgreSQL run directory")?;

    let settings = SettingsBuilder::new()
        .version(V18.clone())
        .installation_dir(install_dir)
        .data_dir(data_dir)
        .password_file(password_file)
        .host("127.0.0.1")
        .username("postgres")
        .password("pontemesh")
        .temporary(true)
        .timeout(Some(Duration::from_secs(60)))
        .config("listen_addresses", "127.0.0.1")
        .build();

    let mut postgres = PostgreSQL::new(settings);
    postgres
        .setup()
        .await
        .context("set up embedded PostgreSQL")?;
    postgres
        .start()
        .await
        .context("start embedded PostgreSQL")?;

    let result = run_s3_parity_test(&postgres).await;
    let stop_result = postgres.stop().await.context("stop embedded PostgreSQL");

    stop_result?;
    result
}

async fn run_s3_parity_test(postgres: &PostgreSQL) -> anyhow::Result<()> {
    postgres
        .create_database(DATABASE_NAME)
        .await
        .context("create test database")?;

    let database_url = postgres.settings().url(DATABASE_NAME);
    let pool = PgPool::connect(&database_url)
        .await
        .context("connect to embedded PostgreSQL test database")?;
    query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(&pool)
        .await
        .context("enable pgcrypto extension")?;
    pool.close().await;

    println!(
        "Running {TEST_NAME} with embedded PostgreSQL at {}:{}",
        postgres.settings().host,
        postgres.settings().port
    );

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .arg("test")
        .arg(TEST_NAME)
        .arg("--")
        .arg("--nocapture")
        .env("TEST_DATABASE_URL", database_url)
        .status()
        .context("run S3 parity test against embedded PostgreSQL")?;

    if !status.success() {
        bail!("S3 parity test failed with status {status}");
    }

    Ok(())
}
