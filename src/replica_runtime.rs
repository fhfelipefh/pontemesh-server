use crate::config::ReplicaRuntimeConfig;
use anyhow::{Context, bail};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tokio::{fs, io::AsyncWriteExt, time};
use tracing::{error, info, warn};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPlanResponse {
    objects: Vec<SyncObject>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncObject {
    bucket: String,
    key: String,
    manifest_id: String,
    size_bytes: i64,
    content_type: String,
    sha256: String,
    state: String,
    fragments: Vec<SyncFragment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncFragment {
    index: i64,
    fragment_id: String,
    byte_range_start: i64,
    byte_range_end: i64,
    size_bytes: i64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PolicyUpdate {
    id: String,
    update_type: String,
    bucket: Option<String>,
    object_key: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ReplicaLocalState {
    objects: HashMap<String, LocalObjectState>,
    last_policy_update_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalObjectState {
    bucket: String,
    key: String,
    manifest_id: String,
    sha256: String,
    size_bytes: i64,
    content_type: String,
    fragments: HashMap<String, LocalFragmentState>,
    synced_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalFragmentState {
    index: i64,
    fragment_id: String,
    sha256: String,
    size_bytes: i64,
    path: String,
    synced_at: String,
}

#[derive(Debug, Default)]
struct SyncStats {
    bytes_synced: i64,
    fragments_synced: i64,
    sync_failures: i64,
    auth_failures: i64,
}

pub async fn run(config: ReplicaRuntimeConfig) -> anyhow::Result<()> {
    fs::create_dir_all(replica_root(&config.storage_path)).await?;
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("failed to create replica HTTP client")?;
    let mut last_health =
        chrono::Utc::now() - chrono::Duration::seconds(config.health_interval_seconds as i64);

    info!(
        replica_id = %config.replica_id,
        origin = %config.origin_base_url,
        "replica-edge runtime started"
    );

    loop {
        let mut stats = SyncStats::default();
        if let Err(error) = sync_once(&client, &config, &mut stats).await {
            stats.sync_failures += 1;
            warn!(error = %error, "replica sync cycle failed");
        }

        if chrono::Utc::now() - last_health
            >= chrono::Duration::seconds(config.health_interval_seconds as i64)
        {
            if let Err(error) = report_health(&client, &config, stats.sync_failures).await {
                warn!(error = %error, "failed to report replica health");
            }
            last_health = chrono::Utc::now();
        }

        if let Err(error) = report_metrics(&client, &config, &stats).await {
            warn!(error = %error, "failed to report replica metrics");
        }

        time::sleep(Duration::from_secs(config.sync_interval_seconds)).await;
    }
}

async fn sync_once(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    stats: &mut SyncStats,
) -> anyhow::Result<()> {
    let mut state = load_state(&config.storage_path).await?;
    apply_policy_updates(client, config, &mut state).await?;
    let plan = signed_json::<SyncPlanResponse>(
        client,
        config,
        Method::GET,
        &format!("/pontemesh/replicas/{}/sync-plan", config.replica_id),
        None,
    )
    .await?;

    let mut active = HashSet::new();
    for object in plan.objects {
        if object.state != "AVAILABLE" {
            continue;
        }
        let object_id = object_key_id(&object.bucket, &object.key);
        active.insert(object_id.clone());
        if object_is_complete(state.objects.get(&object_id), &object) {
            if let Some(local) = state.objects.get(&object_id) {
                announce_availability(client, config, &object, available_fragment_indexes(local))
                    .await?;
            }
            continue;
        }
        match sync_object_fragments(client, config, &mut state, &object).await {
            Ok((bytes_synced, fragments_synced, sync_failures, local)) => {
                stats.bytes_synced += bytes_synced;
                stats.fragments_synced += fragments_synced;
                stats.sync_failures += sync_failures;
                let available_fragments = available_fragment_indexes(&local);
                state.objects.insert(object_id, local);
                if !available_fragments.is_empty() {
                    announce_availability(client, config, &object, available_fragments).await?;
                }
            }
            Err(error) => {
                stats.sync_failures += 1;
                warn!(
                    bucket = %object.bucket,
                    key = %object.key,
                    error = %error,
                    "failed to synchronize object"
                );
            }
        }
    }

    prune_missing_objects(config, &mut state, &active).await?;
    save_state(&config.storage_path, &state).await?;
    Ok(())
}

async fn sync_object_fragments(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    state: &mut ReplicaLocalState,
    object: &SyncObject,
) -> anyhow::Result<(i64, i64, i64, LocalObjectState)> {
    if object.fragments.is_empty() {
        bail!(
            "sync-plan object has no fragments: {}/{}",
            object.bucket,
            object.key
        );
    }

    let existing = state
        .objects
        .remove(&object_key_id(&object.bucket, &object.key));
    let mut local = existing
        .filter(|local| local.manifest_id == object.manifest_id && local.sha256 == object.sha256)
        .unwrap_or_else(|| LocalObjectState {
            bucket: object.bucket.clone(),
            key: object.key.clone(),
            manifest_id: object.manifest_id.clone(),
            sha256: object.sha256.clone(),
            size_bytes: object.size_bytes,
            content_type: object.content_type.clone(),
            fragments: HashMap::new(),
            synced_at: chrono::Utc::now().to_rfc3339(),
        });

    let mut bytes_synced = 0;
    let mut fragments_synced = 0;
    let mut sync_failures = 0;
    let expected_ids = object
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.clone())
        .collect::<HashSet<_>>();
    let stale = local
        .fragments
        .keys()
        .filter(|fragment_id| !expected_ids.contains(*fragment_id))
        .cloned()
        .collect::<Vec<_>>();
    for fragment_id in stale {
        if let Some(fragment) = local.fragments.remove(&fragment_id) {
            remove_fragment_file(config, &fragment).await?;
        }
    }

    for fragment in &object.fragments {
        if local
            .fragments
            .get(&fragment.fragment_id)
            .is_some_and(|local| fragment_is_valid(local, fragment))
        {
            continue;
        }
        if let Some(old_fragment) = local.fragments.remove(&fragment.fragment_id) {
            remove_fragment_file(config, &old_fragment).await?;
        }
        match sync_fragment(client, config, object, fragment).await {
            Ok(synced) => {
                bytes_synced += synced.size_bytes;
                fragments_synced += 1;
                local.fragments.insert(fragment.fragment_id.clone(), synced);
            }
            Err(error) => {
                sync_failures += 1;
                warn!(
                    bucket = %object.bucket,
                    key = %object.key,
                    fragment_id = %fragment.fragment_id,
                    error = %error,
                    "failed to synchronize fragment; keeping existing fragment progress"
                );
            }
        }
    }

    local.synced_at = chrono::Utc::now().to_rfc3339();
    Ok((bytes_synced, fragments_synced, sync_failures, local))
}

async fn sync_fragment(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    object: &SyncObject,
    fragment: &SyncFragment,
) -> anyhow::Result<LocalFragmentState> {
    let path = format!(
        "/pontemesh/replicas/{}/manifests/{}/fragments/{}",
        config.replica_id,
        percent_encode_path_component(&object.manifest_id),
        percent_encode_path_component(&fragment.fragment_id)
    );
    let local_path = local_fragment_path(
        &config.storage_path,
        &object.bucket,
        &object.key,
        &object.manifest_id,
        fragment,
    )?;
    let downloaded =
        signed_download_to_file(client, config, Method::GET, &path, &local_path).await?;
    if downloaded.sha256 != fragment.sha256 {
        let _ = fs::remove_file(&local_path).await;
        bail!(
            "fragment hash mismatch for {}: expected {}, got {}",
            fragment.fragment_id,
            fragment.sha256,
            downloaded.sha256
        );
    }
    if downloaded.size_bytes != fragment.size_bytes {
        let _ = fs::remove_file(&local_path).await;
        bail!(
            "fragment size mismatch for {}: expected {}, got {}",
            fragment.fragment_id,
            fragment.size_bytes,
            downloaded.size_bytes
        );
    }

    Ok(LocalFragmentState {
        index: fragment.index,
        fragment_id: fragment.fragment_id.clone(),
        sha256: fragment.sha256.clone(),
        size_bytes: fragment.size_bytes,
        path: local_path.to_string_lossy().into_owned(),
        synced_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn object_is_complete(local: Option<&LocalObjectState>, object: &SyncObject) -> bool {
    local.is_some_and(|local| {
        local.manifest_id == object.manifest_id
            && local.sha256 == object.sha256
            && object.fragments.iter().all(|fragment| {
                local
                    .fragments
                    .get(&fragment.fragment_id)
                    .is_some_and(|local| fragment_is_valid(local, fragment))
            })
    })
}

fn fragment_is_valid(local: &LocalFragmentState, fragment: &SyncFragment) -> bool {
    local.sha256 == fragment.sha256
        && local.size_bytes == fragment.size_bytes
        && local.index == fragment.index
}

async fn announce_availability(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    object: &SyncObject,
    available_fragments: Vec<i64>,
) -> anyhow::Result<()> {
    let endpoint = format!(
        "{}/objects/{}/{}",
        config.public_endpoint,
        percent_encode_path_component(&object.bucket),
        object_path(&object.key)
    );
    let body = serde_json::json!({
        "bucket": object.bucket,
        "key": object.key,
        "endpoint": endpoint,
        "availableFragments": available_fragments
    });
    signed_json::<serde_json::Value>(
        client,
        config,
        Method::POST,
        &format!("/pontemesh/replicas/{}/availability", config.replica_id),
        Some(body),
    )
    .await?;
    Ok(())
}

async fn report_health(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    sync_failures: i64,
) -> anyhow::Result<()> {
    let body = serde_json::json!({
        "status": if sync_failures == 0 { "OK" } else { "DEGRADED" },
        "version": env!("CARGO_PKG_VERSION"),
        "errorCount": sync_failures,
        "detail": {
            "storagePath": config.storage_path,
            "runtime": "replica-edge"
        }
    });
    signed_json::<serde_json::Value>(
        client,
        config,
        Method::POST,
        &format!("/pontemesh/replicas/{}/health", config.replica_id),
        Some(body),
    )
    .await?;
    Ok(())
}

async fn report_metrics(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    stats: &SyncStats,
) -> anyhow::Result<()> {
    let body = serde_json::json!({
        "bytesSynced": stats.bytes_synced,
        "fragmentsSynced": stats.fragments_synced,
        "syncFailures": stats.sync_failures,
        "authFailures": stats.auth_failures
    });
    signed_json::<serde_json::Value>(
        client,
        config,
        Method::POST,
        &format!("/pontemesh/replicas/{}/metrics", config.replica_id),
        Some(body),
    )
    .await?;
    Ok(())
}

async fn apply_policy_updates(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    state: &mut ReplicaLocalState,
) -> anyhow::Result<()> {
    let mut path = format!("/pontemesh/replicas/{}/policy-updates", config.replica_id);
    if let Some(since) = &state.last_policy_update_at {
        path.push_str("?since=");
        path.push_str(&percent_encode_query_value(since));
    }
    let updates =
        signed_json::<Vec<PolicyUpdate>>(client, config, Method::GET, &path, None).await?;
    for update in updates {
        if update.update_type.contains("revoked") || update.update_type.contains("policy") {
            remove_matching_objects(
                config,
                state,
                update.bucket.as_deref(),
                update.object_key.as_deref(),
            )
            .await?;
        }
        state.last_policy_update_at = Some(update.created_at);
        info!(update_id = %update.id, update_type = %update.update_type, "applied replica policy update");
    }
    Ok(())
}

async fn signed_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    method: Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> anyhow::Result<T> {
    let mut request = signed_request(client, config, method, path);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.context("replica request failed")?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        bail!("replica request rejected with {status}");
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("replica request failed with {status}: {body}");
    }
    response.json::<T>().await.context("invalid JSON response")
}

struct DownloadedFile {
    sha256: String,
    size_bytes: i64,
}

async fn signed_download_to_file(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    method: Method,
    path: &str,
    target: &Path,
) -> anyhow::Result<DownloadedFile> {
    let response = signed_request(client, config, method, path)
        .send()
        .await
        .context("replica object request failed")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("replica object request failed with {status}: {body}");
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }
    let temp_path = target.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let mut file = fs::File::create(&temp_path)
        .await
        .with_context(|| format!("failed to create {}", temp_path.display()))?;
    let mut hasher = Sha256::new();
    let mut size_bytes: i64 = 0;
    let mut response = response;
    loop {
        let chunk = match response.chunk().await {
            Ok(chunk) => chunk,
            Err(error) => {
                let _ = fs::remove_file(&temp_path).await;
                return Err(error).context("failed to read replica response chunk");
            }
        };
        let Some(chunk) = chunk else {
            break;
        };
        let write_result = async {
            size_bytes +=
                i64::try_from(chunk.len()).map_err(|_| anyhow::anyhow!("chunk is too large"))?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp_path).await;
            return Err(error);
        }
    }
    if let Err(error) = file.flush().await {
        let _ = fs::remove_file(&temp_path).await;
        return Err(error).context("failed to flush replica fragment");
    }
    drop(file);
    fs::rename(&temp_path, target).await.with_context(|| {
        format!(
            "failed to move {} to {}",
            temp_path.display(),
            target.display()
        )
    })?;
    Ok(DownloadedFile {
        sha256: format!("{:x}", hasher.finalize()),
        size_bytes,
    })
}

fn signed_request(
    client: &Client,
    config: &ReplicaRuntimeConfig,
    method: Method,
    path: &str,
) -> reqwest::RequestBuilder {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let nonce = uuid::Uuid::new_v4().to_string();
    let signature = replica_signature(
        method.as_str(),
        path,
        &timestamp,
        &nonce,
        &config.replica_token,
    );
    client
        .request(method, format!("{}{}", config.origin_base_url, path))
        .bearer_auth(&config.replica_token)
        .header("x-pontemesh-date", timestamp)
        .header("x-pontemesh-nonce", nonce)
        .header("x-pontemesh-signature", signature)
}

pub fn replica_signature(
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    token: &str,
) -> String {
    let signing_payload = format!("{method}\n{path}\n{timestamp}\n{nonce}");
    hex_hmac(token.as_bytes(), signing_payload.as_bytes())
}

fn hex_hmac(key: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn replica_root(storage_path: &Path) -> PathBuf {
    storage_path.join("replica")
}

fn state_path(storage_path: &Path) -> PathBuf {
    replica_root(storage_path).join("state.json")
}

async fn load_state(storage_path: &Path) -> anyhow::Result<ReplicaLocalState> {
    let path = state_path(storage_path);
    if !path.exists() {
        return Ok(ReplicaLocalState::default());
    }
    let bytes = fs::read(&path).await?;
    serde_json::from_slice(&bytes).context("failed to parse replica local state")
}

async fn save_state(storage_path: &Path, state: &ReplicaLocalState) -> anyhow::Result<()> {
    fs::create_dir_all(replica_root(storage_path)).await?;
    let bytes = serde_json::to_vec_pretty(state)?;
    fs::write(state_path(storage_path), bytes).await?;
    Ok(())
}

async fn prune_missing_objects(
    config: &ReplicaRuntimeConfig,
    state: &mut ReplicaLocalState,
    active: &HashSet<String>,
) -> anyhow::Result<()> {
    let stale = state
        .objects
        .keys()
        .filter(|key| !active.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    for key in stale {
        if let Some(local) = state.objects.remove(&key) {
            remove_local_object(config, &local).await?;
        }
    }
    Ok(())
}

async fn remove_matching_objects(
    config: &ReplicaRuntimeConfig,
    state: &mut ReplicaLocalState,
    bucket: Option<&str>,
    key: Option<&str>,
) -> anyhow::Result<()> {
    let matching = state
        .objects
        .iter()
        .filter(|(_, local)| {
            bucket.is_none_or(|bucket| bucket == local.bucket)
                && key.is_none_or(|key| key == local.key)
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in matching {
        if let Some(local) = state.objects.remove(&id) {
            remove_local_object(config, &local).await?;
        }
    }
    Ok(())
}

async fn remove_local_object(
    config: &ReplicaRuntimeConfig,
    local: &LocalObjectState,
) -> anyhow::Result<()> {
    for fragment in local.fragments.values() {
        remove_fragment_file(config, fragment).await?;
    }
    Ok(())
}

async fn remove_fragment_file(
    config: &ReplicaRuntimeConfig,
    local: &LocalFragmentState,
) -> anyhow::Result<()> {
    let path = PathBuf::from(&local.path);
    if path.starts_with(replica_root(&config.storage_path)) {
        if let Err(error) = fs::remove_file(&path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                error!(path = %path.display(), error = %error, "failed to remove replica local object");
            }
        }
    }
    Ok(())
}

fn local_fragment_path(
    storage_path: &Path,
    bucket: &str,
    key: &str,
    manifest_id: &str,
    fragment: &SyncFragment,
) -> anyhow::Result<PathBuf> {
    let safe_bucket = safe_path_segment(bucket)?;
    let safe_key = safe_object_file_name(key)?;
    let safe_manifest = safe_path_segment(manifest_id)?;
    let prefix = fragment
        .sha256
        .get(0..2)
        .ok_or_else(|| anyhow::anyhow!("fragment hash is too short"))?;
    Ok(replica_root(storage_path)
        .join("fragments")
        .join(safe_bucket)
        .join(safe_key)
        .join(safe_manifest)
        .join(prefix)
        .join(format!("{}-{}", fragment.index, fragment.sha256)))
}

fn safe_path_segment(value: &str) -> anyhow::Result<String> {
    let path = Path::new(value);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("path segment contains unsupported components");
    }
    let sanitized = value.replace(
        |ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_',
        "_",
    );
    if sanitized.is_empty() {
        bail!("path segment cannot be empty");
    }
    Ok(sanitized)
}

fn safe_object_file_name(key: &str) -> anyhow::Result<String> {
    if key.trim().is_empty() || key.starts_with('/') || key.contains('\\') {
        bail!("object key is not safe for local storage");
    }
    let path = Path::new(key);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("object key contains unsupported path components");
    }
    let file = key
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow::anyhow!("object key cannot be empty"))?;
    safe_path_segment(file)
}

fn object_key_id(bucket: &str, key: &str) -> String {
    format!("{bucket}/{key}")
}

fn available_fragment_indexes(local: &LocalObjectState) -> Vec<i64> {
    let mut indexes = local
        .fragments
        .values()
        .map(|fragment| fragment.index)
        .collect::<Vec<_>>();
    indexes.sort_unstable();
    indexes
}

fn object_path(key: &str) -> String {
    key.split('/')
        .map(percent_encode_path_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_query_value(value: &str) -> String {
    percent_encode_path_component(value)
}

fn percent_encode_path_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| {
            let allowed = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
            if allowed {
                vec![byte as char]
            } else {
                format!("%{byte:02X}").chars().collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replica_signature_matches_origin_payload_shape() {
        let signature = replica_signature(
            "GET",
            "/pontemesh/replicas/replica-1/sync-plan",
            "2026-06-30T12:00:00Z",
            "nonce-value-123456",
            "secret-token",
        );
        assert_eq!(signature.len(), 64);
        assert!(signature.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn local_fragment_path_rejects_traversal() {
        let storage = PathBuf::from("/tmp/pontemesh-edge-test");
        let fragment = test_fragment();
        assert!(
            local_fragment_path(&storage, "bucket", "../secret.txt", "manifest-1", &fragment)
                .is_err()
        );
        assert!(
            local_fragment_path(&storage, "bucket", "/secret.txt", "manifest-1", &fragment)
                .is_err()
        );
    }

    #[test]
    fn local_fragment_path_is_inside_replica_root() {
        let storage = PathBuf::from("/tmp/pontemesh-edge-test");
        let fragment = test_fragment();
        let path = local_fragment_path(
            &storage,
            "videos",
            "folder/lesson.mp4",
            "manifest-1",
            &fragment,
        )
        .expect("safe path");
        assert!(path.starts_with(replica_root(&storage)));
        assert!(path.ends_with(format!("7-{}", "a".repeat(64))));
    }

    fn test_fragment() -> SyncFragment {
        SyncFragment {
            index: 7,
            fragment_id: format!("manifest-1:7:{}", "a".repeat(64)),
            byte_range_start: 0,
            byte_range_end: 9,
            size_bytes: 10,
            sha256: "a".repeat(64),
        }
    }
}
