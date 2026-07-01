use crate::{catalog::AuditEventFilter, config, http::AppState, system::storage};
use anyhow::bail;
use serde_json::{Value, json};

pub fn list_tools(write_enabled: bool) -> Value {
    let mut tools = vec![
        tool(
            "pontemesh_get_instance_status",
            "Retorna status geral da instancia.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "pontemesh_get_storage_summary",
            "Retorna resumo do storage configurado.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "pontemesh_list_buckets",
            "Lista buckets com paginacao.",
            paged_schema(json!({})),
        ),
        tool(
            "pontemesh_get_bucket",
            "Consulta um bucket.",
            json!({"type":"object","properties":{"bucket":{"type":"string"}},"required":["bucket"]}),
        ),
        tool(
            "pontemesh_list_objects",
            "Lista objetos de um bucket com paginacao.",
            json!({"type":"object","properties":{"bucket":{"type":"string"},"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":100},"cursor":{"type":"string"}},"required":["bucket"]}),
        ),
        tool(
            "pontemesh_get_object_metadata",
            "Consulta metadados de um objeto.",
            json!({"type":"object","properties":{"bucket":{"type":"string"},"key":{"type":"string"}},"required":["bucket","key"]}),
        ),
        tool(
            "pontemesh_get_health",
            "Retorna verificacoes basicas de saude.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "pontemesh_get_recent_audit_events",
            "Lista eventos recentes de auditoria.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":100}}}),
        ),
    ];
    if !write_enabled {
        tools.push(tool(
            "pontemesh_write_tools_status",
            "Informa que ferramentas de escrita estao desabilitadas.",
            json!({"type":"object","properties":{}}),
        ));
    }
    json!({ "tools": tools })
}

pub async fn call_tool(state: &AppState, name: &str, arguments: Value) -> anyhow::Result<Value> {
    let result = match name {
        "pontemesh_get_instance_status" => {
            json!({
                "name": config::load_instance_config(&state.paths).map(|config| config.instance.name).unwrap_or_else(|_| "Ponte Mesh".to_owned()),
                "role": config::configured_instance_role(&state.paths).ok(),
                "uptimeSeconds": state.started_at.elapsed().as_secs()
            })
        }
        "pontemesh_get_storage_summary" => {
            let path = config::configured_storage_dir(&state.paths)?;
            json!(storage::status(&path))
        }
        "pontemesh_list_buckets" => {
            let (page, page_size) = page_args(&arguments);
            let query = arguments.get("query").and_then(Value::as_str);
            json!(
                state
                    .catalog
                    .list_buckets_page(query, page, page_size)
                    .await?
            )
        }
        "pontemesh_get_bucket" => {
            let bucket = required_str(&arguments, "bucket")?;
            json!(state.catalog.get_bucket(bucket).await?)
        }
        "pontemesh_list_objects" => {
            let bucket = required_str(&arguments, "bucket")?;
            let query = arguments.get("query").and_then(Value::as_str);
            let (page, page_size) = page_args(&arguments);
            json!(
                state
                    .catalog
                    .list_objects_page(bucket, query, page, page_size)
                    .await?
            )
        }
        "pontemesh_get_object_metadata" => {
            let bucket = required_str(&arguments, "bucket")?;
            let key = required_str(&arguments, "key")?;
            json!(state.catalog.get_object_record(bucket, key).await?)
        }
        "pontemesh_get_health" => {
            let storage_path = config::configured_storage_dir(&state.paths)?;
            let storage_status = storage::status(&storage_path);
            json!({
                "databaseConnected": state.catalog.database_connected().await,
                "storageWritable": storage_status.writable,
                "storageWarnings": storage_status.warnings
            })
        }
        "pontemesh_get_recent_audit_events" => {
            let limit = arguments
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(20)
                .clamp(1, 100);
            json!(
                state
                    .catalog
                    .list_audit_events_filtered(AuditEventFilter {
                        event: None,
                        principal: None,
                        outcome: None,
                        since: None,
                        until: None,
                        limit,
                    })
                    .await?
            )
        }
        "pontemesh_write_tools_status" => json!({"writeToolsEnabled": false}),
        _ => bail!("unknown MCP tool: {name}"),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&result)?
        }],
        "structuredContent": result
    }))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn paged_schema(extra: Value) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert("query".to_owned(), json!({"type":"string"}));
    properties.insert(
        "limit".to_owned(),
        json!({"type":"integer","minimum":1,"maximum":100}),
    );
    properties.insert("cursor".to_owned(), json!({"type":"string"}));
    if let Some(extra_properties) = extra.get("properties").and_then(Value::as_object) {
        for (key, value) in extra_properties {
            properties.insert(key.clone(), value.clone());
        }
    }
    json!({"type":"object","properties": properties})
}

fn page_args(arguments: &Value) -> (u32, u32) {
    let page = arguments
        .get("cursor")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 100) as u32;
    (page, page_size)
}

fn required_str<'a>(arguments: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{key} is required"))
}
