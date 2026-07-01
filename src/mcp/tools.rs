use crate::{
    admin::{ConfigurationBackup, ConfigurationMcpSettings},
    catalog::{AuditEventFilter, BucketPolicyUpdate},
    config,
    http::AppState,
    system::storage,
};
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
        tool(
            "pontemesh_export_configuration",
            "Exporta configuracoes operacionais sem segredos.",
            json!({"type":"object","properties":{}}),
        ),
    ];
    if write_enabled {
        tools.push(tool(
            "pontemesh_update_bucket_policy",
            "Atualiza politica hibrida e S3-compatible de um bucket existente.",
            json!({"type":"object","properties":{"bucket":{"type":"string"},"policy":{"type":"object"}},"required":["bucket","policy"]}),
        ));
        tools.push(tool(
            "pontemesh_import_configuration",
            "Importa backup de configuracao operacional sem segredos.",
            json!({"type":"object","properties":{"configuration":{"type":"object"}},"required":["configuration"]}),
        ));
    } else {
        tools.push(tool(
            "pontemesh_write_tools_status",
            "Informa que ferramentas de escrita estao desabilitadas.",
            json!({"type":"object","properties":{}}),
        ));
    }
    json!({ "tools": tools })
}

pub async fn call_tool(
    state: &AppState,
    name: &str,
    arguments: Value,
    write_enabled: bool,
) -> anyhow::Result<Value> {
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
        "pontemesh_export_configuration" => {
            let mcp_settings = state.catalog.get_mcp_settings().await?;
            json!(ConfigurationBackup {
                schema_version: 1,
                exported_at: Some(chrono::Utc::now()),
                mcp_settings: Some(ConfigurationMcpSettings {
                    enabled: mcp_settings.enabled,
                    endpoint_path: mcp_settings.endpoint_path,
                    bind_host: mcp_settings.bind_host,
                    require_auth: mcp_settings.require_auth,
                    read_tools_enabled: mcp_settings.read_tools_enabled,
                    write_tools_enabled: mcp_settings.write_tools_enabled,
                    expose_resources: mcp_settings.expose_resources,
                    expose_prompts: mcp_settings.expose_prompts,
                    allow_localhost_only: mcp_settings.allow_localhost_only,
                }),
                bucket_policies: state.catalog.list_bucket_policies().await?,
            })
        }
        "pontemesh_update_bucket_policy" => {
            ensure_write_enabled(write_enabled)?;
            let bucket = required_str(&arguments, "bucket")?;
            let policy_value = arguments
                .get("policy")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("policy is required"))?;
            let policy: BucketPolicyUpdate = serde_json::from_value(policy_value)?;
            json!(state.catalog.update_bucket_policy(bucket, policy).await?)
        }
        "pontemesh_import_configuration" => {
            ensure_write_enabled(write_enabled)?;
            let configuration_value = arguments
                .get("configuration")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("configuration is required"))?;
            let configuration: ConfigurationBackup = serde_json::from_value(configuration_value)?;
            if configuration.schema_version != 1 {
                bail!("unsupported configuration schemaVersion");
            }
            if let Some(settings) = configuration.mcp_settings {
                state
                    .catalog
                    .update_mcp_settings(crate::catalog::McpSettingsUpdate {
                        enabled: settings.enabled,
                        endpoint_path: settings.endpoint_path,
                        bind_host: settings.bind_host,
                        require_auth: settings.require_auth,
                        read_tools_enabled: settings.read_tools_enabled,
                        write_tools_enabled: settings.write_tools_enabled,
                        expose_resources: settings.expose_resources,
                        expose_prompts: settings.expose_prompts,
                        allow_localhost_only: settings.allow_localhost_only,
                    })
                    .await?;
            }
            let mut applied = 0_usize;
            let mut skipped = Vec::new();
            for policy in configuration.bucket_policies {
                if state.catalog.get_bucket(&policy.bucket_name).await?.is_none() {
                    skipped.push(policy.bucket_name);
                    continue;
                }
                state
                    .catalog
                    .update_bucket_policy(
                        &policy.bucket_name,
                        BucketPolicyUpdate {
                            access_package_ttl_seconds: policy.access_package_ttl_seconds,
                            fragment_size_bytes: policy.fragment_size_bytes,
                            allow_replica_edge: policy.allow_replica_edge,
                            allow_peer_sharing: policy.allow_peer_sharing,
                            source_selection_strategy: policy.source_selection_strategy,
                            fragment_priority_strategy: policy.fragment_priority_strategy,
                            failure_threshold: policy.failure_threshold,
                            fallback_mode: policy.fallback_mode,
                            s3_list_default_max_keys: policy.s3_list_default_max_keys,
                            s3_list_max_keys_limit: policy.s3_list_max_keys_limit,
                            s3_list_allow_delimiter: policy.s3_list_allow_delimiter,
                            s3_versioning_enabled: policy.s3_versioning_enabled,
                            s3_object_tagging_enabled: policy.s3_object_tagging_enabled,
                            s3_checksum_algorithm: policy.s3_checksum_algorithm,
                            s3_multipart_abort_days: policy.s3_multipart_abort_days,
                        },
                    )
                    .await?;
                applied += 1;
            }
            json!({"appliedBucketPolicies": applied, "skippedBucketPolicies": skipped})
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

fn ensure_write_enabled(write_enabled: bool) -> anyhow::Result<()> {
    if write_enabled {
        Ok(())
    } else {
        bail!("MCP write tools are disabled")
    }
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
