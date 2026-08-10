use crate::{
    admin::{ConfigurationBackup, ConfigurationMcpSettings},
    catalog::{self, AuditEventFilter, BucketPolicyUpdate, NewObject},
    config,
    http::AppState,
    setup::agent,
    system::storage,
};
use anyhow::{Context, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MCP_MAX_OBJECT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    Read,
    Write,
    Admin,
}

impl ToolPermission {
    pub fn scope(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

struct ToolDefinition {
    name: &'static str,
    description: &'static str,
    schema: Value,
    permission: ToolPermission,
}

pub fn list_tools(settings: &crate::catalog::McpSettings, scopes: &[String]) -> Value {
    let tools: Vec<Value> = tool_definitions()
        .into_iter()
        .filter(|definition| is_allowed(definition.permission, settings, scopes))
        .map(|definition| tool(definition.name, definition.description, definition.schema))
        .collect();
    json!({ "tools": tools })
}

pub fn tool_permission(name: &str) -> Option<ToolPermission> {
    tool_definitions()
        .into_iter()
        .find(|tool| tool.name == name)
        .map(|tool| tool.permission)
}

pub fn is_allowed(
    permission: ToolPermission,
    settings: &crate::catalog::McpSettings,
    scopes: &[String],
) -> bool {
    let has_scope = scopes.iter().any(|scope| scope == permission.scope())
        || (permission == ToolPermission::Read
            && scopes
                .iter()
                .any(|scope| scope == "write" || scope == "admin"));
    has_scope
        && match permission {
            ToolPermission::Read => settings.read_tools_enabled,
            ToolPermission::Write => settings.write_tools_enabled,
            ToolPermission::Admin => settings.admin_tools_enabled,
        }
}

fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "pontemesh_get_instance_status",
            description: "Retorna status geral da instancia.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_storage_summary",
            description: "Retorna resumo do storage configurado.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_list_buckets",
            description: "Lista buckets com paginacao.",
            schema: paged_schema(json!({})),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_bucket",
            description: "Consulta um bucket.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"}},"required":["bucket"]}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_list_objects",
            description: "Lista objetos de um bucket com paginacao.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":100},"cursor":{"type":"string"}},"required":["bucket"]}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_object_metadata",
            description: "Consulta metadados de um objeto.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"key":{"type":"string"}},"required":["bucket","key"]}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_health",
            description: "Retorna verificacoes basicas de saude.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_recent_audit_events",
            description: "Lista eventos recentes de auditoria.",
            schema: json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":100}}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_export_configuration",
            description: "Exporta configuracoes operacionais sem segredos.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_get_ai_connection_guide",
            description: "Retorna endpoints e instrucoes seguras para clientes de IA, sem expor segredos existentes.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Read,
        },
        ToolDefinition {
            name: "pontemesh_create_bucket",
            description: "Cria um bucket usando o servico real.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"}},"required":["bucket"]}),
            permission: ToolPermission::Write,
        },
        ToolDefinition {
            name: "pontemesh_delete_bucket",
            description: "Remove um bucket vazio.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"}},"required":["bucket"]}),
            permission: ToolPermission::Write,
        },
        ToolDefinition {
            name: "pontemesh_put_text_object",
            description: "Envia objeto textual pequeno via MCP. Para arquivos grandes, use a API S3-compatible.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"key":{"type":"string"},"content":{"type":"string"},"contentType":{"type":"string"}},"required":["bucket","key","content"]}),
            permission: ToolPermission::Write,
        },
        ToolDefinition {
            name: "pontemesh_put_base64_object",
            description: "Envia objeto binario pequeno em base64 via MCP. Para arquivos grandes, use a API S3-compatible.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"key":{"type":"string"},"contentBase64":{"type":"string"},"contentType":{"type":"string"}},"required":["bucket","key","contentBase64"]}),
            permission: ToolPermission::Write,
        },
        ToolDefinition {
            name: "pontemesh_delete_object",
            description: "Apaga objeto usando o servico real.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"key":{"type":"string"}},"required":["bucket","key"]}),
            permission: ToolPermission::Write,
        },
        ToolDefinition {
            name: "pontemesh_update_bucket_policy",
            description: "Atualiza politica hibrida e S3-compatible de um bucket existente.",
            schema: json!({"type":"object","properties":{"bucket":{"type":"string"},"policy":{"type":"object"}},"required":["bucket","policy"]}),
            permission: ToolPermission::Admin,
        },
        ToolDefinition {
            name: "pontemesh_import_configuration",
            description: "Importa backup de configuracao operacional sem segredos.",
            schema: json!({"type":"object","properties":{"configuration":{"type":"object"}},"required":["configuration"]}),
            permission: ToolPermission::Admin,
        },
        ToolDefinition {
            name: "pontemesh_list_credentials",
            description: "Lista credenciais administrativas por metadados seguros, sem segredos completos.",
            schema: json!({"type":"object","properties":{}}),
            permission: ToolPermission::Admin,
        },
        ToolDefinition {
            name: "pontemesh_create_application_credential",
            description: "Cria credencial de aplicacao para SDKs. O token e exibido somente nesta resposta.",
            schema: json!({"type":"object","properties":{"name":{"type":"string"},"scopes":{"type":"array","items":{"type":"string"}}},"required":["name"]}),
            permission: ToolPermission::Admin,
        },
        ToolDefinition {
            name: "pontemesh_create_s3_access_key",
            description: "Cria access key S3. O segredo e exibido somente nesta resposta.",
            schema: json!({"type":"object","properties":{"name":{"type":"string"}}}),
            permission: ToolPermission::Admin,
        },
    ]
}

pub async fn call_tool(state: &AppState, name: &str, arguments: Value) -> anyhow::Result<Value> {
    if matches!(
        tool_permission(name),
        Some(ToolPermission::Write | ToolPermission::Admin)
    ) {
        config::require_instance_role(&state.paths, config::InstanceRole::Origin)?;
    }

    let result = match name {
        "pontemesh_create_bucket" => {
            let bucket = required_str(&arguments, "bucket")?;
            catalog::validate_bucket_name(bucket)?;
            json!(state.catalog.create_bucket(bucket).await?)
        }
        "pontemesh_delete_bucket" => {
            let bucket = required_str(&arguments, "bucket")?;
            state.catalog.delete_bucket(bucket).await?;
            json!({"deleted": true, "bucket": bucket})
        }
        "pontemesh_put_text_object" => {
            let bucket = required_str(&arguments, "bucket")?;
            let key = required_str(&arguments, "key")?;
            let content = required_str(&arguments, "content")?;
            let content_type = arguments
                .get("contentType")
                .and_then(Value::as_str)
                .unwrap_or("text/plain");
            put_small_object(
                state,
                bucket,
                key,
                content.as_bytes().to_vec(),
                content_type,
            )
            .await?
        }
        "pontemesh_put_base64_object" => {
            let bucket = required_str(&arguments, "bucket")?;
            let key = required_str(&arguments, "key")?;
            let encoded = required_str(&arguments, "contentBase64")?;
            if encoded.len() > MCP_MAX_OBJECT_BYTES * 2 {
                bail!("MCP object payload is too large");
            }
            let bytes = BASE64.decode(encoded).context("invalid base64 content")?;
            let content_type = arguments
                .get("contentType")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            put_small_object(state, bucket, key, bytes, content_type).await?
        }
        "pontemesh_delete_object" => {
            let bucket = required_str(&arguments, "bucket")?;
            let key = required_str(&arguments, "key")?;
            state.catalog.delete_object(bucket, key).await?;
            json!({"deleted": true, "bucket": bucket, "key": key})
        }

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
                    .list_objects_page(bucket, query, None, page, page_size)
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
        "pontemesh_get_ai_connection_guide" => {
            let web_addr = config::load_http_bind_addr(&state.paths)?;
            let s3_addr = config::default_s3_bind_addr();
            let mcp_settings = state.catalog.get_mcp_settings().await?;
            json!({
                "webUrl": format!("http://127.0.0.1:{}", web_addr.port()),
                "s3EndpointUrl": format!("http://127.0.0.1:{}", s3_addr.port()),
                "mcp": {
                    "enabled": mcp_settings.enabled,
                    "url": format!("http://127.0.0.1:{}{}", web_addr.port(), mcp_settings.endpoint_path),
                    "method": "POST",
                    "authorization": "Bearer <MCP token>",
                    "localhostOnly": mcp_settings.allow_localhost_only,
                    "readToolsEnabled": mcp_settings.read_tools_enabled,
                    "writeToolsEnabled": mcp_settings.write_tools_enabled,
                    "adminToolsEnabled": mcp_settings.admin_tools_enabled
                },
                "security": {
                    "existingSecretsAreNotReturned": true,
                    "newSecretsAreReturnedOnce": true,
                    "dataPlane": "Object transfer remains on S3-compatible and Ponte Mesh endpoints; MCP is administrative."
                }
            })
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
                    admin_tools_enabled: mcp_settings.admin_tools_enabled,
                    expose_resources: mcp_settings.expose_resources,
                    expose_prompts: mcp_settings.expose_prompts,
                    allow_localhost_only: mcp_settings.allow_localhost_only,
                }),
                bucket_policies: state.catalog.list_bucket_policies().await?,
            })
        }
        "pontemesh_update_bucket_policy" => {
            let bucket = required_str(&arguments, "bucket")?;
            let policy_value = arguments
                .get("policy")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("policy is required"))?;
            let policy: BucketPolicyUpdate = serde_json::from_value(policy_value)?;
            json!(state.catalog.update_bucket_policy(bucket, policy).await?)
        }
        "pontemesh_import_configuration" => {
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
                        admin_tools_enabled: false,
                        expose_resources: settings.expose_resources,
                        expose_prompts: settings.expose_prompts,
                        allow_localhost_only: settings.allow_localhost_only,
                    })
                    .await?;
            }
            let mut applied = 0_usize;
            let mut skipped = Vec::new();
            for policy in configuration.bucket_policies {
                if state
                    .catalog
                    .get_bucket(&policy.bucket_name)
                    .await?
                    .is_none()
                {
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
                            s3_default_encryption_algorithm: policy.s3_default_encryption_algorithm,
                            s3_default_encryption_key_id: policy.s3_default_encryption_key_id,
                            s3_object_lock_enabled: policy.s3_object_lock_enabled,
                            s3_object_lock_default_mode: policy.s3_object_lock_default_mode,
                            s3_object_lock_default_retain_days: policy
                                .s3_object_lock_default_retain_days,
                            s3_lifecycle_rules: policy.s3_lifecycle_rules,
                            s3_resource_policy: policy.s3_resource_policy,
                            s3_event_notifications: policy.s3_event_notifications,
                        },
                    )
                    .await?;
                applied += 1;
            }
            json!({"appliedBucketPolicies": applied, "skippedBucketPolicies": skipped})
        }
        "pontemesh_list_credentials" => {
            json!({
                "mcpTokens": state.catalog.list_mcp_access_tokens().await?,
                "applicationCredentials": state.catalog.list_application_credentials().await?,
                "s3AccessKeys": state.catalog.list_s3_access_keys(1, 100).await?,
                "secretsIncluded": false
            })
        }
        "pontemesh_create_application_credential" => {
            let name = required_str(&arguments, "name")?;
            let scopes = optional_string_array(&arguments, "scopes")?
                .unwrap_or_else(default_application_scopes);
            let created = state
                .catalog
                .create_application_credential(name, scopes)
                .await?;
            state
                .catalog
                .record_audit_event(
                    "application_credential_created",
                    Some("mcp"),
                    "success",
                    &format!("application_id={}", created.credential.id),
                )
                .await?;
            json!(created)
        }
        "pontemesh_create_s3_access_key" => {
            let name = arguments.get("name").and_then(Value::as_str);
            json!(agent::create_s3_key_for_mcp(state, name).await?)
        }
        _ => bail!("unknown MCP tool: {name}"),
    };
    Ok(
        json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&result)?}],"structuredContent":result}),
    )
}

async fn put_small_object(
    state: &AppState,
    bucket: &str,
    key: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> anyhow::Result<Value> {
    catalog::validate_bucket_name(bucket)?;
    catalog::validate_object_key(key)?;
    if bytes.len() > MCP_MAX_OBJECT_BYTES {
        bail!(
            "MCP object payload exceeds 1 MiB limit; use the S3-compatible API for large uploads"
        );
    }
    let policy = state.catalog.get_bucket_policy(bucket).await?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = storage_path.join(bucket);
    tokio::fs::create_dir_all(&bucket_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create bucket storage directory {}",
                bucket_dir.display()
            )
        })?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let object_path = bucket_dir.join(format!("{}-{}", uuid::Uuid::new_v4(), sha256));
    tokio::fs::write(&object_path, &bytes)
        .await
        .with_context(|| format!("failed to write MCP object data {}", object_path.display()))?;
    let object = NewObject {
        bucket_name: bucket.to_owned(),
        key: key.to_owned(),
        size_bytes: i64::try_from(bytes.len()).context("object too large")?,
        content_type: content_type.to_owned(),
        sha256: sha256.clone(),
        storage_path: object_path.display().to_string(),
        checksum_sha256: None,
        checksum_crc32: None,
        encryption_algorithm: None,
        encryption_key_id: None,
        encryption_nonce: None,
        object_lock_mode: None,
        retain_until: None,
        legal_hold: false,
        manifest: build_manifest(&bytes, policy.fragment_size_bytes)?,
    };
    match state
        .catalog
        .put_object_with_audit(
            object,
            "mcp",
            &format!("bucket={bucket}; key={key}; source=mcp"),
        )
        .await
    {
        Ok(summary) => Ok(json!(summary)),
        Err(error) => {
            let _ = tokio::fs::remove_file(&object_path).await;
            Err(error)
        }
    }
}

fn build_manifest(
    bytes: &[u8],
    fragment_size_bytes: i64,
) -> anyhow::Result<catalog::NewObjectManifest> {
    if fragment_size_bytes <= 0 {
        bail!("fragmentSizeBytes must be positive");
    }
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    let fragments = bytes
        .chunks(fragment_size)
        .enumerate()
        .map(|(index, chunk)| {
            let start = index
                .checked_mul(fragment_size)
                .and_then(|v| i64::try_from(v).ok())
                .ok_or_else(|| anyhow::anyhow!("fragment byte range is too large"))?;
            let size_bytes =
                i64::try_from(chunk.len()).context("fragment size cannot fit in i64")?;
            Ok(catalog::NewObjectFragment {
                index: i64::try_from(index).context("fragment index cannot fit in i64")?,
                byte_range_start: start,
                byte_range_end: start + size_bytes.saturating_sub(1),
                size_bytes,
                sha256: format!("{:x}", Sha256::digest(chunk)),
                priority: if index == 0 {
                    "INITIAL".to_owned()
                } else {
                    "NORMAL".to_owned()
                },
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(catalog::NewObjectManifest {
        fragment_size_bytes,
        fragments,
    })
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
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
    json!({"type":"object","properties":properties})
}

fn page_args(arguments: &Value) -> (u32, u32) {
    let page = arguments
        .get("page")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 10_000) as u32;
    let page_size = arguments
        .get("limit")
        .or_else(|| arguments.get("pageSize"))
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 100) as u32;
    (page, page_size)
}

fn required_str<'a>(arguments: &'a Value, name: &str) -> anyhow::Result<&'a str> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn optional_string_array(arguments: &Value, name: &str) -> anyhow::Result<Option<Vec<String>>> {
    let Some(value) = arguments.get(name) else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{name} must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("{name} must contain non-empty strings"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.is_empty() {
        bail!("{name} must include at least one value");
    }
    Ok(Some(values))
}

fn default_application_scopes() -> Vec<String> {
    vec![
        "origin:objects:read".to_owned(),
        "origin:objects:write".to_owned(),
        "pontemesh:access-package:create".to_owned(),
        "pontemesh:manifest:read".to_owned(),
        "pontemesh:sources:read".to_owned(),
        "pontemesh:availability:read".to_owned(),
        "pontemesh:policies:read".to_owned(),
    ]
}
