use crate::{
    audit,
    config::{self, PontemeshHome, WebhookSection},
    system::{disk_guard, storage},
};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use cron::Schedule;
use reqwest::{Client, Url, redirect::Policy};
use serde::Serialize;
use std::{str::FromStr, time::Duration};
use tokio::time::sleep;
use tracing::warn;

const WEBHOOK_POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalWebhookPayload {
    schema_version: u8,
    event: &'static str,
    generated_at: DateTime<Utc>,
    instance: WebhookInstance,
    storage: WebhookStorage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebhookInstance {
    name: String,
    role: config::InstanceRole,
    version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebhookStorage {
    path: String,
    exists: bool,
    writable: bool,
    total_bytes: Option<u64>,
    available_bytes: Option<u64>,
    used_bytes: Option<u64>,
    used_percent: Option<f64>,
    guard: disk_guard::DiskGuardStatus,
    warnings: Vec<String>,
}

pub fn validate(section: WebhookSection) -> anyhow::Result<WebhookSection> {
    let cron = section.cron.trim().to_owned();
    parse_schedule(&cron)?;
    let url = section
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(validate_url)
        .transpose()?
        .map(|url| url.to_string());
    if section.enabled && url.is_none() {
        bail!("webhook URL is required when delivery is enabled");
    }
    Ok(WebhookSection {
        enabled: section.enabled,
        url,
        cron,
    })
}

pub fn build_payload(paths: &PontemeshHome) -> anyhow::Result<OperationalWebhookPayload> {
    let config = config::load_instance_config(paths)?;
    let storage_path = config::configured_storage_dir(paths)?;
    let storage_status = storage::status(&storage_path);
    let guard = disk_guard::check(&storage_path, &config.storage.guards);
    Ok(OperationalWebhookPayload {
        schema_version: 1,
        event: "pontemesh.operational_status",
        generated_at: Utc::now(),
        instance: WebhookInstance {
            name: config.instance.name,
            role: config.instance.role,
            version: env!("CARGO_PKG_VERSION"),
        },
        storage: WebhookStorage {
            path: storage_status.path,
            exists: storage_status.exists,
            writable: storage_status.writable,
            total_bytes: storage_status.total_bytes,
            available_bytes: storage_status.available_bytes,
            used_bytes: storage_status.used_bytes,
            used_percent: storage_status.used_percent,
            guard,
            warnings: storage_status.warnings,
        },
    })
}

pub async fn run(paths: PontemeshHome) {
    let client = match Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(Policy::limited(3))
        .user_agent("pontemesh-server-operational-webhook")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            warn!(%error, "operational webhook client could not be created");
            return;
        }
    };
    let mut last_checked = Utc::now();
    loop {
        sleep(WEBHOOK_POLL_INTERVAL).await;
        let now = Utc::now();
        if deliver_if_due(&client, &paths, last_checked, now)
            .await
            .is_err()
        {
            warn!("operational webhook delivery failed");
            audit::failure(
                "operational_webhook_delivery_failed",
                None,
                "delivery failed",
            );
        }
        last_checked = now;
    }
}

async fn deliver_if_due(
    client: &Client,
    paths: &PontemeshHome,
    last_checked: DateTime<Utc>,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let config = config::load_instance_config(paths)?;
    let webhook = validate(config.webhook)?;
    let Some(url) = webhook.url.filter(|_| webhook.enabled) else {
        return Ok(());
    };
    let schedule = parse_schedule(&webhook.cron)?;
    if !is_due(&schedule, last_checked, now) {
        return Ok(());
    }
    let payload = build_payload(paths)?;
    client
        .post(url)
        .json(&payload)
        .send()
        .await
        .context("webhook request failed")?
        .error_for_status()
        .context("webhook endpoint returned an error")?;
    audit::event(
        "operational_webhook_delivered",
        None,
        "success",
        "operational status delivered",
    );
    Ok(())
}

fn validate_url(value: &str) -> anyhow::Result<Url> {
    if value.len() > 2048 {
        bail!("webhook URL cannot exceed 2048 characters");
    }
    let url = Url::parse(value).context("webhook URL is invalid")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        bail!("webhook URL must use HTTP or HTTPS and include a host");
    }
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        bail!("webhook URL cannot contain credentials or a fragment");
    }
    Ok(url)
}

fn parse_schedule(expression: &str) -> anyhow::Result<Schedule> {
    if expression.split_whitespace().count() != 5 {
        bail!("cron must contain exactly five fields");
    }
    Schedule::from_str(&format!("0 {expression}")).with_context(|| "cron expression is invalid")
}

fn is_due(schedule: &Schedule, last_checked: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    schedule
        .after(&last_checked)
        .next()
        .is_some_and(|next| next <= now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn accepts_five_field_cron_and_http_destinations() {
        let section = validate(WebhookSection {
            enabled: true,
            url: Some("http://localhost:5678/webhook/storage".to_owned()),
            cron: "*/15 * * * *".to_owned(),
        })
        .expect("valid webhook");

        assert_eq!(
            section.url.as_deref(),
            Some("http://localhost:5678/webhook/storage")
        );
    }

    #[test]
    fn rejects_credentials_and_invalid_cron() {
        assert!(validate_url("https://user:secret@example.test/hook").is_err());
        assert!(parse_schedule("every minute").is_err());
    }

    #[test]
    fn detects_a_schedule_between_poll_boundaries() {
        let schedule = parse_schedule("*/15 * * * *").expect("schedule");
        let last = Utc.with_ymd_and_hms(2026, 8, 11, 12, 14, 50).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 8, 11, 12, 15, 20).unwrap();

        assert!(is_due(&schedule, last, now));
    }
}
