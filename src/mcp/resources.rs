use crate::{catalog::AuditEventFilter, config, http::AppState, system::storage};
use anyhow::bail;
use serde_json::{Value, json};

pub fn list_resources() -> Value {
    json!({
        "resources": [
            resource("pontemesh://instance/status", "Instance status"),
            resource("pontemesh://instance/health", "Instance health"),
            resource("pontemesh://storage/summary", "Storage summary"),
            resource("pontemesh://buckets", "Buckets"),
            resource("pontemesh://audit/recent", "Recent audit events")
        ]
    })
}

pub async fn read_resource(state: &AppState, uri: &str) -> anyhow::Result<Value> {
    let value = match uri {
        "pontemesh://instance/status" => json!({
            "name": config::load_instance_config(&state.paths).map(|config| config.instance.name).unwrap_or_else(|_| "Ponte Mesh".to_owned()),
            "role": config::configured_instance_role(&state.paths).ok(),
            "uptimeSeconds": state.started_at.elapsed().as_secs()
        }),
        "pontemesh://instance/health" => {
            let storage_path = config::configured_storage_dir(&state.paths)?;
            let storage_status = storage::status(&storage_path);
            json!({
                "databaseConnected": state.catalog.database_connected().await,
                "storageWritable": storage_status.writable,
                "storageWarnings": storage_status.warnings
            })
        }
        "pontemesh://storage/summary" => {
            let storage_path = config::configured_storage_dir(&state.paths)?;
            json!(storage::status(&storage_path))
        }
        "pontemesh://buckets" => json!(state.catalog.list_buckets_page(None, 1, 100).await?),
        "pontemesh://audit/recent" => json!(
            state
                .catalog
                .list_audit_events_filtered(AuditEventFilter {
                    event: None,
                    principal: None,
                    outcome: None,
                    since: None,
                    until: None,
                    limit: 50,
                })
                .await?
        ),
        _ if uri.starts_with("pontemesh://buckets/") && uri.ends_with("/objects") => {
            let bucket = uri
                .trim_start_matches("pontemesh://buckets/")
                .trim_end_matches("/objects");
            json!(
                state
                    .catalog
                    .list_objects_page(bucket, None, 1, 100)
                    .await?
            )
        }
        _ if uri.starts_with("pontemesh://buckets/") => {
            let bucket = uri.trim_start_matches("pontemesh://buckets/");
            json!(state.catalog.get_bucket(bucket).await?)
        }
        _ => bail!("unknown MCP resource: {uri}"),
    };
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&value)?
        }]
    }))
}

fn resource(uri: &str, name: &str) -> Value {
    json!({
        "uri": uri,
        "name": name,
        "mimeType": "application/json"
    })
}
