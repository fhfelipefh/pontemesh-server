use crate::{
    audit,
    catalog::{
        self, BucketSummary, ListObjectsV2Options, MultipartPartRecord, MultipartUploadRecord,
        NewObject, NewObjectFragment, NewObjectManifest, ObjectRecord, ObjectSummary,
        S3ListObjectsPage, S3ObjectTag,
    },
    config,
    http::AppState,
    s3_auth::S3Identity,
};
use anyhow::{Context, bail};
use axum::{
    Extension,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use http_body_util::BodyExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{cmp, fs, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    #[serde(rename = "list-type")]
    list_type: Option<String>,
    prefix: Option<String>,
    delimiter: Option<String>,
    #[serde(rename = "max-keys")]
    max_keys: Option<i64>,
    #[serde(rename = "continuation-token")]
    continuation_token: Option<String>,
    #[serde(rename = "start-after")]
    start_after: Option<String>,
    uploads: Option<String>,
    location: Option<String>,
    delete: Option<String>,
    versioning: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ObjectMultipartQuery {
    #[serde(rename = "uploadId")]
    upload_id: Option<String>,
    #[serde(rename = "partNumber")]
    part_number: Option<i32>,
    uploads: Option<String>,
    tagging: Option<String>,
}

pub async fn list_buckets(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
) -> Response {
    match state.catalog.list_buckets().await {
        Ok(buckets) => {
            record_origin_audit(
                &state,
                "s3_list_buckets",
                &identity.access_key_id,
                "success",
                "list buckets",
            )
            .await;
            s3_xml_response(StatusCode::OK, list_buckets_xml(&buckets))
        }
        Err(error) => s3_internal_error(error),
    }
}

async fn create_bucket_inner_response(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
) -> Response {
    match create_bucket_inner(&state, &bucket_name).await {
        Ok(bucket) => {
            audit::event(
                "s3_bucket_created",
                Some(&identity.access_key_id),
                "success",
                &format!("bucket={}", bucket.name),
            );
            record_origin_audit(
                &state,
                "s3_bucket_created",
                &identity.access_key_id,
                "success",
                &format!("bucket={}", bucket.name),
            )
            .await;
            Response::builder()
                .status(StatusCode::OK)
                .header("x-amz-request-id", request_id())
                .header("Location", format!("/{}", bucket.name))
                .body(Body::empty())
                .expect("valid CreateBucket response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

pub async fn head_bucket(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.get_bucket(&bucket_name).await {
        Ok(Some(_)) => Response::builder()
            .status(StatusCode::OK)
            .header("x-amz-request-id", request_id())
            .body(Body::empty())
            .expect("valid HeadBucket response"),
        Ok(None) => s3_error(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist",
            Some(&bucket_name),
            None,
        ),
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

pub async fn list_objects(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
    Query(query): Query<ListObjectsQuery>,
) -> Response {
    if query.location.is_some() {
        return match state.catalog.get_bucket(&bucket_name).await {
            Ok(Some(_)) => s3_xml_response(StatusCode::OK, bucket_location_xml()),
            Ok(None) => s3_error(
                StatusCode::NOT_FOUND,
                "NoSuchBucket",
                "The specified bucket does not exist",
                Some(&bucket_name),
                None,
            ),
            Err(error) => s3_bad_request(error, Some(&bucket_name), None),
        };
    }

    if query.versioning.is_some() {
        return match state.catalog.get_bucket_policy(&bucket_name).await {
            Ok(policy) => s3_xml_response(
                StatusCode::OK,
                bucket_versioning_xml(policy.s3_versioning_enabled),
            ),
            Err(error) => s3_bad_request(error, Some(&bucket_name), None),
        };
    }

    if query.uploads.is_some() {
        return match state
            .catalog
            .list_active_multipart_uploads(&bucket_name)
            .await
        {
            Ok(uploads) => s3_xml_response(
                StatusCode::OK,
                list_multipart_uploads_xml(&bucket_name, &uploads),
            ),
            Err(error) => s3_bad_request(error, Some(&bucket_name), None),
        };
    }

    if query.list_type.as_deref() != Some("2") {
        return s3_error(
            StatusCode::BAD_REQUEST,
            "InvalidArgument",
            "Only ListObjectsV2 is supported",
            Some(&bucket_name),
            None,
        );
    }

    let policy = match state.catalog.get_bucket_policy(&bucket_name).await {
        Ok(policy) => policy,
        Err(error) => return s3_bad_request(error, Some(&bucket_name), None),
    };
    let max_keys = query
        .max_keys
        .unwrap_or(policy.s3_list_default_max_keys)
        .clamp(1, policy.s3_list_max_keys_limit);
    let delimiter = query
        .delimiter
        .clone()
        .filter(|value| policy.s3_list_allow_delimiter && value == "/");
    let options = ListObjectsV2Options {
        prefix: query.prefix.clone(),
        delimiter,
        max_keys,
        continuation_token: query.continuation_token.clone(),
        start_after: query.start_after.clone(),
    };

    match state.catalog.list_objects_v2(&bucket_name, options).await {
        Ok(page) => s3_xml_response(StatusCode::OK, list_objects_v2_xml(&bucket_name, &query, &page)),
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

pub async fn post_bucket(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    Path(bucket_name): Path<String>,
    Query(query): Query<ListObjectsQuery>,
    body: Body,
) -> Response {
    if query.delete.is_some() {
        return delete_objects(state, identity, bucket_name, body).await;
    }

    s3_error(
        StatusCode::BAD_REQUEST,
        "InvalidArgument",
        "POST Bucket supports only DeleteObjects",
        Some(&bucket_name),
        None,
    )
}

pub async fn put_bucket(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    Path(bucket_name): Path<String>,
    Query(query): Query<ListObjectsQuery>,
    body: Body,
) -> Response {
    if query.versioning.is_some() {
        return put_bucket_versioning(state, identity, bucket_name, body).await;
    }

    create_bucket_inner_response(state, identity, bucket_name).await
}

pub async fn put_object(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    headers: HeaderMap,
    Path((bucket_name, object_key)): Path<(String, String)>,
    Query(query): Query<ObjectMultipartQuery>,
    body: Body,
) -> Response {
    if query.tagging.is_some() {
        return put_object_tagging(state, identity, bucket_name, object_key, body).await;
    }

    if let (Some(upload_id), Some(part_number)) = (query.upload_id.as_deref(), query.part_number) {
        return upload_multipart_part(
            state,
            identity,
            bucket_name,
            object_key,
            upload_id,
            part_number,
            body,
        )
        .await;
    }

    if headers.contains_key("x-amz-copy-source") {
        return copy_object(state, identity, headers, bucket_name, object_key).await;
    }

    let request_id = request_id();
    info!(
        bucket = %bucket_name,
        object_key = %object_key,
        request_id = %request_id,
        "put_object_started"
    );
    match put_object_inner(
        &state,
        &identity.access_key_id,
        &bucket_name,
        &object_key,
        &headers,
        body,
        &request_id,
    )
    .await
    {
        Ok(object) => {
            audit::event(
                "s3_object_put",
                Some(&identity.access_key_id),
                "success",
                &format!("bucket={bucket_name}; key={}", object.key),
            );
            info!(
                bucket = %bucket_name,
                object_key = %object.key,
                size_bytes = object.size_bytes,
                request_id = %request_id,
                "put_object_completed"
            );
            Response::builder()
                .status(StatusCode::OK)
                .header("ETag", format!("\"{}\"", object.sha256))
                .header("x-amz-request-id", request_id)
                .header(header::CONTENT_LENGTH, "0")
                .body(Body::empty())
                .expect("valid PutObject response")
        }
        Err(error) => {
            warn!(
                bucket = %bucket_name,
                object_key = %object_key,
                request_id = %request_id,
                error = %error,
                "put_object_failed"
            );
            s3_bad_request(error, Some(&bucket_name), Some(&object_key))
        }
    }
}

pub async fn post_object(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    headers: HeaderMap,
    Path((bucket_name, object_key)): Path<(String, String)>,
    Query(query): Query<ObjectMultipartQuery>,
    body: Body,
) -> Response {
    if let Some(upload_id) = query.upload_id.as_deref() {
        return complete_multipart_upload(
            state,
            identity,
            bucket_name,
            object_key,
            upload_id,
            body,
        )
        .await;
    }

    if query.uploads.is_some() {
        return initiate_multipart_upload(state, identity, headers, bucket_name, object_key).await;
    }

    s3_error(
        StatusCode::BAD_REQUEST,
        "InvalidArgument",
        "POST Object supports only multipart upload initiation or completion",
        Some(&bucket_name),
        Some(&object_key),
    )
}

pub async fn head_object(
    State(state): State<AppState>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    match state
        .catalog
        .get_object_record(&bucket_name, &object_key)
        .await
    {
        Ok(Some(object)) if object.state == "AVAILABLE" => object_metadata_response(&object, true),
        Ok(Some(_)) => s3_error(
            StatusCode::FORBIDDEN,
            "InvalidObjectState",
            "The object is not available",
            Some(&bucket_name),
            Some(&object_key),
        ),
        Ok(None) => s3_error(
            StatusCode::NOT_FOUND,
            "NoSuchKey",
            "The specified key does not exist",
            Some(&bucket_name),
            Some(&object_key),
        ),
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

pub async fn get_object(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    headers: HeaderMap,
    Path((bucket_name, object_key)): Path<(String, String)>,
    Query(query): Query<ObjectMultipartQuery>,
) -> Response {
    if query.tagging.is_some() {
        return get_object_tagging(state, identity, bucket_name, object_key).await;
    }

    if let Some(upload_id) = query.upload_id.as_deref() {
        return match state
            .catalog
            .list_multipart_parts(&bucket_name, &object_key, upload_id)
            .await
        {
            Ok(parts) => s3_xml_response(
                StatusCode::OK,
                list_parts_xml(&bucket_name, &object_key, upload_id, &parts),
            ),
            Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
        };
    }

    match get_object_inner(&state, &bucket_name, &object_key, &headers).await {
        Ok(served) => {
            record_origin_audit(
                &state,
                "s3_object_get",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            if let Err(error) = state
                .catalog
                .record_origin_transfer(
                    None,
                    &bucket_name,
                    &object_key,
                    served.bytes_served,
                    served.range,
                    served.status_code,
                )
                .await
            {
                audit::failure(
                    "origin_transfer_metric_failed",
                    Some(&identity.access_key_id),
                    &error.to_string(),
                );
            }
            served.response
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

pub async fn delete_object(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    Query(query): Query<ObjectMultipartQuery>,
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
    if query.tagging.is_some() {
        return delete_object_tagging(state, identity, bucket_name, object_key).await;
    }

    if let Some(upload_id) = query.upload_id.as_deref() {
        return abort_multipart_upload(state, identity, bucket_name, object_key, upload_id).await;
    }

    match state.catalog.delete_object(&bucket_name, &object_key).await {
        Ok(()) => {
            audit::event(
                "s3_object_deleted",
                Some(&identity.access_key_id),
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            );
            record_origin_audit(
                &state,
                "s3_object_deleted",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("x-amz-request-id", request_id())
                .body(Body::empty())
                .expect("valid DeleteObject response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

pub async fn delete_bucket(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    Path(bucket_name): Path<String>,
) -> Response {
    match state.catalog.delete_bucket(&bucket_name).await {
        Ok(()) => {
            record_origin_audit(
                &state,
                "s3_bucket_deleted",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("x-amz-request-id", request_id())
                .body(Body::empty())
                .expect("valid DeleteBucket response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

async fn initiate_multipart_upload(
    state: AppState,
    identity: S3Identity,
    headers: HeaderMap,
    bucket_name: String,
    object_key: String,
) -> Response {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("application/octet-stream");
    match state
        .catalog
        .create_multipart_upload(
            &bucket_name,
            &object_key,
            content_type,
            &identity.access_key_id,
        )
        .await
    {
        Ok(upload) => {
            record_origin_audit(
                &state,
                "s3_multipart_upload_initiated",
                &identity.access_key_id,
                "success",
                &format!(
                    "bucket={bucket_name}; key={object_key}; upload_id={}",
                    upload.upload_id
                ),
            )
            .await;
            s3_xml_response(StatusCode::OK, initiate_multipart_upload_xml(&upload))
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn put_bucket_versioning(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    body: Body,
) -> Response {
    let enabled = match parse_bucket_versioning(body).await {
        Ok(enabled) => enabled,
        Err(error) => return s3_bad_request(error, Some(&bucket_name), None),
    };
    let current = match state.catalog.get_bucket_policy(&bucket_name).await {
        Ok(policy) => policy,
        Err(error) => return s3_bad_request(error, Some(&bucket_name), None),
    };
    let update = catalog::BucketPolicyUpdate {
        access_package_ttl_seconds: current.access_package_ttl_seconds,
        fragment_size_bytes: current.fragment_size_bytes,
        allow_replica_edge: current.allow_replica_edge,
        allow_peer_sharing: current.allow_peer_sharing,
        source_selection_strategy: current.source_selection_strategy,
        fragment_priority_strategy: current.fragment_priority_strategy,
        failure_threshold: current.failure_threshold,
        fallback_mode: current.fallback_mode,
        s3_list_default_max_keys: current.s3_list_default_max_keys,
        s3_list_max_keys_limit: current.s3_list_max_keys_limit,
        s3_list_allow_delimiter: current.s3_list_allow_delimiter,
        s3_versioning_enabled: enabled,
        s3_object_tagging_enabled: current.s3_object_tagging_enabled,
        s3_checksum_algorithm: current.s3_checksum_algorithm,
        s3_multipart_abort_days: current.s3_multipart_abort_days,
    };
    match state.catalog.update_bucket_policy(&bucket_name, update).await {
        Ok(_) => {
            record_origin_audit(
                &state,
                "s3_bucket_versioning_updated",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; enabled={enabled}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::OK)
                .header("x-amz-request-id", request_id())
                .header(header::CONTENT_LENGTH, "0")
                .body(Body::empty())
                .expect("valid PutBucketVersioning response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

async fn get_object_tagging(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
) -> Response {
    match object_tagging_enabled(&state, &bucket_name).await {
        Ok(true) => {}
        Ok(false) => return tagging_disabled_response(&bucket_name, &object_key),
        Err(error) => return s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
    match state.catalog.list_object_tags(&bucket_name, &object_key).await {
        Ok(tags) => {
            record_origin_audit(
                &state,
                "s3_object_tags_read",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            s3_xml_response(StatusCode::OK, object_tagging_xml(&tags))
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn put_object_tagging(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
    body: Body,
) -> Response {
    match object_tagging_enabled(&state, &bucket_name).await {
        Ok(true) => {}
        Ok(false) => return tagging_disabled_response(&bucket_name, &object_key),
        Err(error) => return s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
    let tags = match parse_object_tagging(body).await {
        Ok(tags) => tags,
        Err(error) => return s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    };
    match state
        .catalog
        .replace_object_tags(&bucket_name, &object_key, tags)
        .await
    {
        Ok(_) => {
            record_origin_audit(
                &state,
                "s3_object_tags_updated",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::OK)
                .header("x-amz-request-id", request_id())
                .header(header::CONTENT_LENGTH, "0")
                .body(Body::empty())
                .expect("valid PutObjectTagging response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn delete_object_tagging(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
) -> Response {
    match object_tagging_enabled(&state, &bucket_name).await {
        Ok(true) => {}
        Ok(false) => return tagging_disabled_response(&bucket_name, &object_key),
        Err(error) => return s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
    match state.catalog.delete_object_tags(&bucket_name, &object_key).await {
        Ok(()) => {
            record_origin_audit(
                &state,
                "s3_object_tags_deleted",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("x-amz-request-id", request_id())
                .body(Body::empty())
                .expect("valid DeleteObjectTagging response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn upload_multipart_part(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
    upload_id: &str,
    part_number: i32,
    body: Body,
) -> Response {
    match upload_multipart_part_inner(
        &state,
        &bucket_name,
        &object_key,
        upload_id,
        part_number,
        body,
    )
    .await
    {
        Ok(part) => {
            record_origin_audit(
                &state,
                "s3_multipart_part_uploaded",
                &identity.access_key_id,
                "success",
                &format!(
                    "bucket={bucket_name}; key={object_key}; upload_id={upload_id}; part_number={}",
                    part.part_number
                ),
            )
            .await;
            Response::builder()
                .status(StatusCode::OK)
                .header("ETag", format!("\"{}\"", part.etag))
                .header("x-amz-request-id", request_id())
                .header(header::CONTENT_LENGTH, "0")
                .body(Body::empty())
                .expect("valid UploadPart response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn complete_multipart_upload(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
    upload_id: &str,
    body: Body,
) -> Response {
    match complete_multipart_upload_inner(
        &state,
        &identity.access_key_id,
        &bucket_name,
        &object_key,
        upload_id,
        body,
    )
    .await
    {
        Ok(object) => {
            record_origin_audit(
                &state,
                "s3_multipart_upload_completed",
                &identity.access_key_id,
                "success",
                &format!(
                    "bucket={bucket_name}; key={}; upload_id={upload_id}",
                    object.key
                ),
            )
            .await;
            s3_xml_response(
                StatusCode::OK,
                complete_multipart_upload_xml(&bucket_name, &object.key, &object.sha256),
            )
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn abort_multipart_upload(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    object_key: String,
    upload_id: &str,
) -> Response {
    let parts = state
        .catalog
        .list_multipart_parts(&bucket_name, &object_key, upload_id)
        .await
        .unwrap_or_default();
    match state.catalog.abort_multipart_upload(upload_id).await {
        Ok(()) => {
            remove_multipart_part_files(&parts);
            record_origin_audit(
                &state,
                "s3_multipart_upload_aborted",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={object_key}; upload_id={upload_id}"),
            )
            .await;
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("x-amz-request-id", request_id())
                .body(Body::empty())
                .expect("valid AbortMultipartUpload response")
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn copy_object(
    state: AppState,
    identity: S3Identity,
    headers: HeaderMap,
    bucket_name: String,
    object_key: String,
) -> Response {
    match copy_object_inner(
        &state,
        &identity.access_key_id,
        &bucket_name,
        &object_key,
        &headers,
    )
    .await
    {
        Ok(object) => {
            record_origin_audit(
                &state,
                "s3_object_copied",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; key={}", object.key),
            )
            .await;
            s3_xml_response(StatusCode::OK, copy_object_xml(&object))
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), Some(&object_key)),
    }
}

async fn delete_objects(
    state: AppState,
    identity: S3Identity,
    bucket_name: String,
    body: Body,
) -> Response {
    match delete_objects_inner(&state, &identity.access_key_id, &bucket_name, body).await {
        Ok(result) => {
            record_origin_audit(
                &state,
                "s3_objects_deleted",
                &identity.access_key_id,
                "success",
                &format!("bucket={bucket_name}; deleted={}", result.deleted.len()),
            )
            .await;
            s3_xml_response(StatusCode::OK, delete_objects_xml(&result))
        }
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

async fn create_bucket_inner(state: &AppState, bucket_name: &str) -> anyhow::Result<BucketSummary> {
    catalog::validate_bucket_name(bucket_name)?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    fs::create_dir_all(bucket_storage_dir(storage_path, bucket_name))
        .with_context(|| format!("failed to create storage directory for bucket {bucket_name}"))?;
    state.catalog.create_bucket(bucket_name).await
}

async fn put_object_inner(
    state: &AppState,
    principal: &str,
    bucket_name: &str,
    object_key: &str,
    headers: &HeaderMap,
    body: Body,
    request_id: &str,
) -> anyhow::Result<ObjectSummary> {
    catalog::validate_bucket_name(bucket_name)?;
    catalog::validate_object_key(object_key)?;
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;

    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = bucket_storage_dir(storage_path, bucket_name);
    let temp_dir = config::configured_storage_dir(&state.paths)?.join("tmp/uploads");
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "failed to create temporary upload directory {}",
            temp_dir.display()
        )
    })?;
    fs::create_dir_all(&bucket_dir).with_context(|| {
        format!(
            "failed to create bucket storage directory {}",
            bucket_dir.display()
        )
    })?;
    let upload_id = uuid::Uuid::new_v4();
    let temp_path = temp_dir.join(format!("{upload_id}.tmp"));
    let streamed = match write_body_to_temp_object(
        body,
        &temp_path,
        policy.fragment_size_bytes,
        bucket_name,
        object_key,
        request_id,
    )
    .await
    {
        Ok(streamed) => streamed,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    let object_path = bucket_dir.join(format!("{}-{}", upload_id, streamed.sha256));
    if let Err(error) = fs::rename(&temp_path, &object_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("failed to move object data {}", object_path.display()));
    }
    info!(
        bucket = %bucket_name,
        object_key = %object_key,
        size_bytes = streamed.size_bytes,
        request_id = %request_id,
        "put_object_storage_written"
    );

    let detail = format!("bucket={bucket_name}; key={object_key}");
    let object = match state
        .catalog
        .put_object_with_audit(
            NewObject {
                bucket_name: bucket_name.to_owned(),
                key: object_key.trim_start_matches('/').to_owned(),
                size_bytes: streamed.size_bytes,
                content_type,
                sha256: streamed.sha256,
                storage_path: object_path.display().to_string(),
                manifest: streamed.manifest,
            },
            principal,
            &detail,
        )
        .await
    {
        Ok(object) => object,
        Err(error) => {
            let _ = fs::remove_file(&object_path);
            return Err(error);
        }
    };
    info!(
        bucket = %bucket_name,
        object_key = %object_key,
        size_bytes = object.size_bytes,
        request_id = %request_id,
        "put_object_catalog_saved"
    );
    Ok(object)
}

async fn upload_multipart_part_inner(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
    upload_id: &str,
    part_number: i32,
    body: Body,
) -> anyhow::Result<MultipartPartRecord> {
    if !(1..=10_000).contains(&part_number) {
        bail!("partNumber must be between 1 and 10000");
    }
    let upload = state
        .catalog
        .get_active_multipart_upload(bucket_name, object_key, upload_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("multipart upload not found or no longer active"))?;
    let multipart_dir = multipart_upload_dir(state, &upload.upload_id)?;
    fs::create_dir_all(&multipart_dir).with_context(|| {
        format!(
            "failed to create multipart directory {}",
            multipart_dir.display()
        )
    })?;
    let part_path = multipart_dir.join(format!("part-{part_number:05}"));
    let written = write_body_to_multipart_part(body, &part_path).await?;
    let old_parts = state
        .catalog
        .list_multipart_parts(bucket_name, object_key, upload_id)
        .await?;
    let old_path = old_parts
        .iter()
        .find(|part| part.part_number == part_number)
        .map(|part| part.storage_path.clone());
    let part = state
        .catalog
        .record_multipart_part(
            bucket_name,
            object_key,
            upload_id,
            MultipartPartRecord {
                part_number,
                etag: written.sha256,
                size_bytes: written.size_bytes,
                storage_path: part_path.display().to_string(),
                uploaded_at: String::new(),
            },
        )
        .await?;
    if let Some(old_path) = old_path.filter(|old_path| old_path != &part.storage_path) {
        let _ = fs::remove_file(old_path);
    }
    Ok(part)
}

async fn complete_multipart_upload_inner(
    state: &AppState,
    principal: &str,
    bucket_name: &str,
    object_key: &str,
    upload_id: &str,
    body: Body,
) -> anyhow::Result<ObjectSummary> {
    let upload = state
        .catalog
        .get_active_multipart_upload(bucket_name, object_key, upload_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("multipart upload not found or no longer active"))?;
    let requested_parts = parse_complete_multipart_upload(body).await?;
    let stored_parts = state
        .catalog
        .list_multipart_parts(bucket_name, object_key, upload_id)
        .await?;
    let selected_parts = resolve_completed_parts(&requested_parts, &stored_parts)?;
    let policy = state.catalog.get_bucket_policy(bucket_name).await?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = bucket_storage_dir(storage_path, bucket_name);
    let temp_dir = config::configured_storage_dir(&state.paths)?.join("tmp/uploads");
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "failed to create temporary upload directory {}",
            temp_dir.display()
        )
    })?;
    fs::create_dir_all(&bucket_dir).with_context(|| {
        format!(
            "failed to create bucket storage directory {}",
            bucket_dir.display()
        )
    })?;
    let temp_path = temp_dir.join(format!("{upload_id}-complete.tmp"));
    let assembled =
        assemble_multipart_object(&selected_parts, &temp_path, policy.fragment_size_bytes).await?;
    let object_path = bucket_dir.join(format!("{}-{}", upload_id, assembled.sha256));
    if let Err(error) = fs::rename(&temp_path, &object_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("failed to move object data {}", object_path.display()));
    }
    let detail = format!("bucket={bucket_name}; key={object_key}; upload_id={upload_id}");
    let object = match state
        .catalog
        .put_object_with_audit(
            NewObject {
                bucket_name: bucket_name.to_owned(),
                key: object_key.trim_start_matches('/').to_owned(),
                size_bytes: assembled.size_bytes,
                content_type: upload.content_type,
                sha256: assembled.sha256,
                storage_path: object_path.display().to_string(),
                manifest: assembled.manifest,
            },
            principal,
            &detail,
        )
        .await
    {
        Ok(object) => object,
        Err(error) => {
            let _ = fs::remove_file(&object_path);
            return Err(error);
        }
    };
    if let Err(error) = state.catalog.complete_multipart_upload(upload_id).await {
        let _ = fs::remove_file(&object_path);
        return Err(error);
    }
    remove_multipart_part_files(&stored_parts);
    Ok(object)
}

async fn copy_object_inner(
    state: &AppState,
    principal: &str,
    destination_bucket: &str,
    destination_key: &str,
    headers: &HeaderMap,
) -> anyhow::Result<ObjectSummary> {
    catalog::validate_bucket_name(destination_bucket)?;
    catalog::validate_object_key(destination_key)?;
    let copy_source = headers
        .get("x-amz-copy-source")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("x-amz-copy-source header is required"))?;
    let (source_bucket, source_key) = parse_copy_source(copy_source)?;
    let source = state
        .catalog
        .get_object_record(&source_bucket, &source_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("copy source object not found"))?;
    if source.state != "AVAILABLE" {
        bail!("copy source object is not available");
    }
    let policy = state.catalog.get_bucket_policy(destination_bucket).await?;
    let storage_path = config::configured_storage_dir(&state.paths)?;
    let bucket_dir = bucket_storage_dir(storage_path, destination_bucket);
    let temp_dir = config::configured_storage_dir(&state.paths)?.join("tmp/uploads");
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "failed to create temporary upload directory {}",
            temp_dir.display()
        )
    })?;
    fs::create_dir_all(&bucket_dir).with_context(|| {
        format!(
            "failed to create bucket storage directory {}",
            bucket_dir.display()
        )
    })?;
    let copy_id = uuid::Uuid::new_v4();
    let temp_path = temp_dir.join(format!("{copy_id}-copy.tmp"));
    let copied = match copy_file_to_temp_object(
        &source.storage_path,
        &temp_path,
        policy.fragment_size_bytes,
    )
    .await
    {
        Ok(copied) => copied,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    let object_path = bucket_dir.join(format!("{}-{}", copy_id, copied.sha256));
    if let Err(error) = fs::rename(&temp_path, &object_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("failed to move copied object {}", object_path.display()));
    }
    let detail = format!(
        "source_bucket={source_bucket}; source_key={source_key}; bucket={destination_bucket}; key={destination_key}"
    );
    match state
        .catalog
        .put_object_with_audit(
            NewObject {
                bucket_name: destination_bucket.to_owned(),
                key: destination_key.trim_start_matches('/').to_owned(),
                size_bytes: copied.size_bytes,
                content_type: source.content_type,
                sha256: copied.sha256,
                storage_path: object_path.display().to_string(),
                manifest: copied.manifest,
            },
            principal,
            &detail,
        )
        .await
    {
        Ok(object) => Ok(object),
        Err(error) => {
            let _ = fs::remove_file(&object_path);
            Err(error)
        }
    }
}

struct DeleteObjectsResult {
    deleted: Vec<String>,
    errors: Vec<DeleteObjectError>,
}

struct DeleteObjectError {
    key: String,
    code: String,
    message: String,
}

async fn delete_objects_inner(
    state: &AppState,
    principal: &str,
    bucket_name: &str,
    body: Body,
) -> anyhow::Result<DeleteObjectsResult> {
    catalog::validate_bucket_name(bucket_name)?;
    if state.catalog.get_bucket(bucket_name).await?.is_none() {
        bail!("bucket not found: {bucket_name}");
    }
    let keys = parse_delete_objects(body).await?;
    let mut deleted = Vec::new();
    let mut errors = Vec::new();
    for key in keys {
        match state.catalog.get_object_record(bucket_name, &key).await {
            Ok(Some(_)) => match state.catalog.delete_object(bucket_name, &key).await {
                Ok(()) => deleted.push(key),
                Err(error) => errors.push(DeleteObjectError {
                    key,
                    code: "InternalError".to_owned(),
                    message: error.to_string(),
                }),
            },
            Ok(None) => deleted.push(key),
            Err(error) => errors.push(DeleteObjectError {
                key,
                code: "InvalidRequest".to_owned(),
                message: error.to_string(),
            }),
        }
    }
    audit::event(
        "s3_objects_deleted",
        Some(principal),
        "success",
        &format!("bucket={bucket_name}; deleted={}", deleted.len()),
    );
    Ok(DeleteObjectsResult { deleted, errors })
}

struct StreamedObject {
    size_bytes: i64,
    sha256: String,
    manifest: NewObjectManifest,
}

struct WrittenPart {
    size_bytes: i64,
    sha256: String,
}

async fn write_body_to_temp_object(
    mut body: Body,
    temp_path: &PathBuf,
    fragment_size_bytes: i64,
    bucket_name: &str,
    object_key: &str,
    request_id: &str,
) -> anyhow::Result<StreamedObject> {
    if fragment_size_bytes <= 0 {
        bail!("fragmentSizeBytes must be positive");
    }
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    let mut file = tokio::fs::File::create(temp_path)
        .await
        .with_context(|| format!("failed to create temporary upload {}", temp_path.display()))?;
    let mut object_hasher = Sha256::new();
    let mut fragment_hasher = Sha256::new();
    let mut fragments = Vec::new();
    let mut fragment_len = 0usize;
    let mut fragment_start = 0i64;
    let mut size_bytes = 0i64;

    while let Some(frame) = body.frame().await {
        let frame = frame.context("failed to read request body")?;
        let Some(chunk) = frame.data_ref() else {
            continue;
        };
        if chunk.is_empty() {
            continue;
        }
        file.write_all(chunk)
            .await
            .with_context(|| format!("failed to write upload chunk {}", temp_path.display()))?;
        object_hasher.update(chunk);

        let mut remaining = chunk.as_ref();
        while !remaining.is_empty() {
            let capacity = fragment_size - fragment_len;
            let take = cmp::min(capacity, remaining.len());
            fragment_hasher.update(&remaining[..take]);
            fragment_len += take;
            size_bytes = size_bytes
                .checked_add(i64::try_from(take).context("uploaded object is too large")?)
                .context("uploaded object is too large")?;
            remaining = &remaining[take..];

            if fragment_len == fragment_size {
                push_streamed_fragment(
                    &mut fragments,
                    &mut fragment_hasher,
                    &mut fragment_start,
                    &mut fragment_len,
                )?;
            }
        }
    }

    if fragment_len > 0 {
        push_streamed_fragment(
            &mut fragments,
            &mut fragment_hasher,
            &mut fragment_start,
            &mut fragment_len,
        )?;
    }

    file.flush()
        .await
        .with_context(|| format!("failed to flush upload {}", temp_path.display()))?;
    drop(file);

    let sha256 = format!("{:x}", object_hasher.finalize());
    info!(
        bucket = %bucket_name,
        object_key = %object_key,
        size_bytes,
        request_id = %request_id,
        "put_object_body_received"
    );
    Ok(StreamedObject {
        size_bytes,
        sha256,
        manifest: NewObjectManifest {
            fragment_size_bytes,
            fragments,
        },
    })
}

async fn write_body_to_multipart_part(
    mut body: Body,
    part_path: &PathBuf,
) -> anyhow::Result<WrittenPart> {
    let mut file = tokio::fs::File::create(part_path)
        .await
        .with_context(|| format!("failed to create multipart part {}", part_path.display()))?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0i64;
    while let Some(frame) = body.frame().await {
        let frame = frame.context("failed to read multipart part body")?;
        let Some(chunk) = frame.data_ref() else {
            continue;
        };
        if chunk.is_empty() {
            continue;
        }
        file.write_all(chunk)
            .await
            .with_context(|| format!("failed to write multipart part {}", part_path.display()))?;
        hasher.update(chunk);
        size_bytes = size_bytes
            .checked_add(i64::try_from(chunk.len()).context("multipart part is too large")?)
            .context("multipart part is too large")?;
    }
    file.flush()
        .await
        .with_context(|| format!("failed to flush multipart part {}", part_path.display()))?;
    Ok(WrittenPart {
        size_bytes,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

async fn assemble_multipart_object(
    parts: &[MultipartPartRecord],
    temp_path: &PathBuf,
    fragment_size_bytes: i64,
) -> anyhow::Result<StreamedObject> {
    if fragment_size_bytes <= 0 {
        bail!("fragmentSizeBytes must be positive");
    }
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    let mut output = tokio::fs::File::create(temp_path)
        .await
        .with_context(|| format!("failed to create completed upload {}", temp_path.display()))?;
    let mut object_hasher = Sha256::new();
    let mut fragment_hasher = Sha256::new();
    let mut fragments = Vec::new();
    let mut fragment_len = 0usize;
    let mut fragment_start = 0i64;
    let mut size_bytes = 0i64;
    let mut buffer = vec![0u8; 1024 * 1024];

    for part in parts {
        let mut file = tokio::fs::File::open(&part.storage_path)
            .await
            .with_context(|| format!("failed to open multipart part {}", part.storage_path))?;
        loop {
            let read = file
                .read(&mut buffer)
                .await
                .with_context(|| format!("failed to read multipart part {}", part.storage_path))?;
            if read == 0 {
                break;
            }
            let chunk = &buffer[..read];
            output.write_all(chunk).await.with_context(|| {
                format!("failed to write completed upload {}", temp_path.display())
            })?;
            object_hasher.update(chunk);

            let mut remaining = chunk;
            while !remaining.is_empty() {
                let capacity = fragment_size - fragment_len;
                let take = cmp::min(capacity, remaining.len());
                fragment_hasher.update(&remaining[..take]);
                fragment_len += take;
                size_bytes = size_bytes
                    .checked_add(i64::try_from(take).context("completed object is too large")?)
                    .context("completed object is too large")?;
                remaining = &remaining[take..];
                if fragment_len == fragment_size {
                    push_streamed_fragment(
                        &mut fragments,
                        &mut fragment_hasher,
                        &mut fragment_start,
                        &mut fragment_len,
                    )?;
                }
            }
        }
    }

    if fragment_len > 0 {
        push_streamed_fragment(
            &mut fragments,
            &mut fragment_hasher,
            &mut fragment_start,
            &mut fragment_len,
        )?;
    }

    output
        .flush()
        .await
        .with_context(|| format!("failed to flush completed upload {}", temp_path.display()))?;
    Ok(StreamedObject {
        size_bytes,
        sha256: format!("{:x}", object_hasher.finalize()),
        manifest: NewObjectManifest {
            fragment_size_bytes,
            fragments,
        },
    })
}

async fn copy_file_to_temp_object(
    source_path: &str,
    temp_path: &PathBuf,
    fragment_size_bytes: i64,
) -> anyhow::Result<StreamedObject> {
    if fragment_size_bytes <= 0 {
        bail!("fragmentSizeBytes must be positive");
    }
    let fragment_size =
        usize::try_from(fragment_size_bytes).context("fragment size is too large")?;
    let mut input = tokio::fs::File::open(source_path)
        .await
        .with_context(|| format!("failed to open copy source {source_path}"))?;
    let mut output = tokio::fs::File::create(temp_path)
        .await
        .with_context(|| format!("failed to create copy temp {}", temp_path.display()))?;
    let mut object_hasher = Sha256::new();
    let mut fragment_hasher = Sha256::new();
    let mut fragments = Vec::new();
    let mut fragment_len = 0usize;
    let mut fragment_start = 0i64;
    let mut size_bytes = 0i64;
    let mut buffer = vec![0u8; 1024 * 1024];

    loop {
        let read = input
            .read(&mut buffer)
            .await
            .with_context(|| format!("failed to read copy source {source_path}"))?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        output
            .write_all(chunk)
            .await
            .with_context(|| format!("failed to write copy temp {}", temp_path.display()))?;
        object_hasher.update(chunk);

        let mut remaining = chunk;
        while !remaining.is_empty() {
            let capacity = fragment_size - fragment_len;
            let take = cmp::min(capacity, remaining.len());
            fragment_hasher.update(&remaining[..take]);
            fragment_len += take;
            size_bytes = size_bytes
                .checked_add(i64::try_from(take).context("copied object is too large")?)
                .context("copied object is too large")?;
            remaining = &remaining[take..];
            if fragment_len == fragment_size {
                push_streamed_fragment(
                    &mut fragments,
                    &mut fragment_hasher,
                    &mut fragment_start,
                    &mut fragment_len,
                )?;
            }
        }
    }

    if fragment_len > 0 {
        push_streamed_fragment(
            &mut fragments,
            &mut fragment_hasher,
            &mut fragment_start,
            &mut fragment_len,
        )?;
    }
    output
        .flush()
        .await
        .with_context(|| format!("failed to flush copy temp {}", temp_path.display()))?;
    Ok(StreamedObject {
        size_bytes,
        sha256: format!("{:x}", object_hasher.finalize()),
        manifest: NewObjectManifest {
            fragment_size_bytes,
            fragments,
        },
    })
}

fn push_streamed_fragment(
    fragments: &mut Vec<NewObjectFragment>,
    fragment_hasher: &mut Sha256,
    fragment_start: &mut i64,
    fragment_len: &mut usize,
) -> anyhow::Result<()> {
    let size_bytes = i64::try_from(*fragment_len).context("fragment size cannot fit in i64")?;
    if size_bytes == 0 {
        return Ok(());
    }
    let index = i64::try_from(fragments.len()).context("fragment index cannot fit in i64")?;
    let byte_range_start = *fragment_start;
    let byte_range_end = byte_range_start
        .checked_add(size_bytes.saturating_sub(1))
        .context("fragment byte range is too large")?;
    let sha256 = format!("{:x}", fragment_hasher.finalize_reset());
    fragments.push(NewObjectFragment {
        index,
        byte_range_start,
        byte_range_end,
        size_bytes,
        sha256,
        priority: if index == 0 {
            "INITIAL".to_owned()
        } else {
            "NORMAL".to_owned()
        },
    });
    *fragment_start = byte_range_end
        .checked_add(1)
        .context("fragment byte range is too large")?;
    *fragment_len = 0;
    Ok(())
}

#[derive(Debug)]
struct CompletedPartRequest {
    part_number: i32,
    etag: String,
}

async fn parse_complete_multipart_upload(body: Body) -> anyhow::Result<Vec<CompletedPartRequest>> {
    let bytes = body
        .collect()
        .await
        .context("failed to read CompleteMultipartUpload body")?
        .to_bytes();
    let xml = std::str::from_utf8(&bytes).context("CompleteMultipartUpload body is not UTF-8")?;
    let mut parts = Vec::new();
    for part_block in xml.split("<Part>").skip(1) {
        let part_block = part_block
            .split_once("</Part>")
            .map(|(part, _)| part)
            .ok_or_else(|| anyhow::anyhow!("invalid CompleteMultipartUpload XML"))?;
        let part_number = xml_tag_value(part_block, "PartNumber")?
            .parse::<i32>()
            .context("PartNumber is invalid")?;
        let etag = xml_tag_value(part_block, "ETag")?
            .trim_matches('"')
            .to_owned();
        parts.push(CompletedPartRequest { part_number, etag });
    }
    if parts.is_empty() {
        bail!("CompleteMultipartUpload must include at least one Part");
    }
    Ok(parts)
}

async fn parse_bucket_versioning(body: Body) -> anyhow::Result<bool> {
    let bytes = body
        .collect()
        .await
        .context("failed to read PutBucketVersioning body")?
        .to_bytes();
    let xml = std::str::from_utf8(&bytes).context("PutBucketVersioning body is not UTF-8")?;
    let status = extract_xml_tag(xml, "Status").unwrap_or_default();
    match status.as_str() {
        "Enabled" => Ok(true),
        "Suspended" | "" => Ok(false),
        _ => bail!("Bucket versioning Status must be Enabled or Suspended"),
    }
}

async fn parse_object_tagging(body: Body) -> anyhow::Result<Vec<S3ObjectTag>> {
    let bytes = body
        .collect()
        .await
        .context("failed to read PutObjectTagging body")?
        .to_bytes();
    let xml = std::str::from_utf8(&bytes).context("PutObjectTagging body is not UTF-8")?;
    let mut tags = Vec::new();
    for tag_xml in extract_xml_blocks(xml, "Tag") {
        let key = extract_xml_tag(&tag_xml, "Key")
            .ok_or_else(|| anyhow::anyhow!("Tag must include Key"))?;
        let value = extract_xml_tag(&tag_xml, "Value").unwrap_or_default();
        tags.push(S3ObjectTag { key, value });
    }
    Ok(tags)
}

async fn parse_delete_objects(body: Body) -> anyhow::Result<Vec<String>> {
    let bytes = body
        .collect()
        .await
        .context("failed to read DeleteObjects body")?
        .to_bytes();
    let xml = std::str::from_utf8(&bytes).context("DeleteObjects body is not UTF-8")?;
    let mut keys = Vec::new();
    for object_block in xml.split("<Object>").skip(1) {
        let object_block = object_block
            .split_once("</Object>")
            .map(|(object, _)| object)
            .ok_or_else(|| anyhow::anyhow!("invalid DeleteObjects XML"))?;
        keys.push(xml_unescape(xml_tag_value(object_block, "Key")?));
    }
    if keys.is_empty() {
        bail!("DeleteObjects must include at least one Object");
    }
    Ok(keys)
}

fn xml_tag_value<'a>(xml: &'a str, tag: &str) -> anyhow::Result<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let value = xml
        .split_once(&open)
        .and_then(|(_, rest)| rest.split_once(&close).map(|(value, _)| value))
        .ok_or_else(|| anyhow::anyhow!("missing XML tag {tag}"))?;
    Ok(value.trim())
}

fn parse_copy_source(raw: &str) -> anyhow::Result<(String, String)> {
    let raw = raw.trim().trim_start_matches('/');
    let decoded = percent_decode(raw);
    let (bucket, key) = decoded
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("x-amz-copy-source must include bucket and key"))?;
    catalog::validate_bucket_name(bucket)?;
    catalog::validate_object_key(key)?;
    Ok((bucket.to_owned(), key.to_owned()))
}

fn resolve_completed_parts(
    requested: &[CompletedPartRequest],
    stored: &[MultipartPartRecord],
) -> anyhow::Result<Vec<MultipartPartRecord>> {
    let mut selected = Vec::with_capacity(requested.len());
    let mut last_part_number = 0;
    for request in requested {
        if request.part_number <= last_part_number {
            bail!("multipart parts must be completed in ascending partNumber order");
        }
        last_part_number = request.part_number;
        let part = stored
            .iter()
            .find(|part| part.part_number == request.part_number)
            .ok_or_else(|| anyhow::anyhow!("multipart part is missing"))?;
        if part.etag != request.etag {
            bail!("multipart part ETag does not match uploaded part");
        }
        selected.push(part.clone());
    }
    Ok(selected)
}

async fn get_object_inner(
    state: &AppState,
    bucket_name: &str,
    object_key: &str,
    headers: &HeaderMap,
) -> anyhow::Result<ServedObjectResponse> {
    let object = state
        .catalog
        .get_object_record(bucket_name, object_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("object not found"))?;
    if object.state != "AVAILABLE" {
        bail!("object is not available");
    }
    let bytes = fs::read(&object.storage_path)
        .with_context(|| format!("failed to read object data {}", object.storage_path))?;

    let total_size = bytes.len() as u64;
    let Some(range_header) = headers.get(header::RANGE) else {
        let bytes_served = i64::try_from(bytes.len()).context("object response is too large")?;
        return Ok(ServedObjectResponse {
            response: object_body_response(&object, StatusCode::OK, bytes, None),
            bytes_served,
            range: None,
            status_code: StatusCode::OK.as_u16(),
        });
    };

    let range_header = range_header
        .to_str()
        .context("Range header is not valid UTF-8")?;
    let range = parse_range(range_header, total_size)?;
    let start = usize::try_from(range.start).context("range start is too large")?;
    let end = usize::try_from(range.end).context("range end is too large")?;
    let partial = bytes[start..=end].to_vec();
    let bytes_served = i64::try_from(partial.len()).context("object response is too large")?;
    Ok(ServedObjectResponse {
        response: object_body_response(&object, StatusCode::PARTIAL_CONTENT, partial, Some(range)),
        bytes_served,
        range: Some((range.start, range.end)),
        status_code: StatusCode::PARTIAL_CONTENT.as_u16(),
    })
}

struct ServedObjectResponse {
    response: Response,
    bytes_served: i64,
    range: Option<(u64, u64)>,
    status_code: u16,
}

fn object_metadata_response(object: &ObjectRecord, head_only: bool) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, object.content_type.as_str())
        .header(header::CONTENT_LENGTH, object.size_bytes.to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header("ETag", format!("\"{}\"", object.sha256))
        .header("Last-Modified", object.created_at.as_str())
        .header("x-amz-request-id", request_id())
        .header("x-amz-bucket-region", "us-east-1")
        .header("x-pontemesh-object-state", object.state.as_str())
        .header("x-pontemesh-created-at", object.created_at.as_str());

    if head_only {
        builder = builder.header("x-pontemesh-object-key", object.key.as_str());
    }

    builder
        .body(Body::empty())
        .expect("valid object metadata response")
}

fn object_body_response(
    object: &ObjectRecord,
    status: StatusCode,
    bytes: Vec<u8>,
    range: Option<ResolvedRange>,
) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, object.content_type.as_str())
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header("ETag", format!("\"{}\"", object.sha256))
        .header("Last-Modified", object.created_at.as_str())
        .header("x-amz-request-id", request_id())
        .header("x-amz-bucket-region", "us-east-1")
        .header("x-pontemesh-object-state", object.state.as_str());

    if let Some(range) = range {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", range.start, range.end, object.size_bytes),
        );
    }

    builder
        .body(Body::from(bytes))
        .expect("valid object body response")
}

#[derive(Debug, Clone, Copy)]
struct ResolvedRange {
    start: u64,
    end: u64,
}

fn parse_range(raw: &str, total_size: u64) -> anyhow::Result<ResolvedRange> {
    if total_size == 0 {
        bail!("cannot apply Range to empty object");
    }
    let range = raw
        .strip_prefix("bytes=")
        .ok_or_else(|| anyhow::anyhow!("only bytes ranges are supported"))?;
    if range.contains(',') {
        bail!("multiple ranges are not supported");
    }
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| anyhow::anyhow!("invalid Range header"))?;

    let (start, end) = if start.is_empty() {
        let suffix_len: u64 = end.parse().context("invalid suffix byte range")?;
        if suffix_len == 0 {
            bail!("suffix byte range must be greater than zero");
        }
        let start = total_size.saturating_sub(suffix_len);
        (start, total_size - 1)
    } else {
        let start: u64 = start.parse().context("invalid range start")?;
        let end = if end.is_empty() {
            total_size - 1
        } else {
            end.parse().context("invalid range end")?
        };
        (start, end)
    };

    if start >= total_size || end >= total_size || start > end {
        bail!("requested range is not satisfiable");
    }

    Ok(ResolvedRange { start, end })
}

fn bucket_storage_dir(storage_path: PathBuf, bucket_name: &str) -> PathBuf {
    storage_path.join("buckets").join(bucket_name)
}

fn s3_bad_request(error: anyhow::Error, bucket: Option<&str>, key: Option<&str>) -> Response {
    let message = error.to_string();
    let (status, code, user_message) = if message == "requested range is not satisfiable" {
        (
            StatusCode::RANGE_NOT_SATISFIABLE,
            "InvalidRange",
            "The requested range is not satisfiable",
        )
    } else if message == "object not found" || message.starts_with("object not found") {
        (
            StatusCode::NOT_FOUND,
            "NoSuchKey",
            "The specified key does not exist",
        )
    } else if message.starts_with("bucket not found") {
        (
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist",
        )
    } else if message.contains("bucket must be empty") {
        (
            StatusCode::CONFLICT,
            "BucketNotEmpty",
            "The bucket you tried to delete is not empty",
        )
    } else if message.contains("already exists") {
        (
            StatusCode::CONFLICT,
            "BucketAlreadyOwnedByYou",
            "The requested bucket already exists",
        )
    } else if message == "object is not available" {
        (
            StatusCode::FORBIDDEN,
            "InvalidObjectState",
            "The object is not available",
        )
    } else {
        (StatusCode::BAD_REQUEST, "InvalidRequest", message.as_str())
    };
    let mut response = s3_error(status, code, user_message, bucket, key);
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        response
            .headers_mut()
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }
    response
}

fn s3_internal_error(error: anyhow::Error) -> Response {
    s3_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "InternalError",
        &error.to_string(),
        None,
        None,
    )
}

fn s3_xml_response(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .header(header::CONTENT_LENGTH, body.len().to_string())
        .header("x-amz-request-id", request_id())
        .body(Body::from(body))
        .expect("valid S3 XML response")
}

fn s3_error(
    status: StatusCode,
    code: &str,
    message: &str,
    bucket: Option<&str>,
    key: Option<&str>,
) -> Response {
    let mut extra = String::new();
    if let Some(bucket) = bucket {
        extra.push_str("<BucketName>");
        extra.push_str(&xml_escape(bucket));
        extra.push_str("</BucketName>");
    }
    if let Some(key) = key {
        extra.push_str("<Key>");
        extra.push_str(&xml_escape(key));
        extra.push_str("</Key>");
    }
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Error><Code>{}</Code><Message>{}</Message>{}<RequestId>{}</RequestId></Error>",
        xml_escape(code),
        xml_escape(message),
        extra,
        request_id()
    );
    s3_xml_response(status, body)
}

fn list_buckets_xml(buckets: &[BucketSummary]) -> String {
    let buckets_xml = buckets
        .iter()
        .map(|bucket| {
            format!(
                "<Bucket><Name>{}</Name><CreationDate>{}</CreationDate></Bucket>",
                xml_escape(&bucket.name),
                xml_escape(&bucket.created_at)
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<ListAllMyBucketsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Owner><ID>pontemesh</ID><DisplayName>Ponte Mesh</DisplayName></Owner>\
<Buckets>{buckets_xml}</Buckets></ListAllMyBucketsResult>"
    )
}

fn list_objects_v2_xml(bucket_name: &str, query: &ListObjectsQuery, page: &S3ListObjectsPage) -> String {
    let prefix = query.prefix.as_deref().unwrap_or("");
    let contents = page
        .items
        .iter()
        .map(|object| {
            format!(
                "<Contents><Key>{}</Key><LastModified>{}</LastModified>\
<ETag>&quot;{}&quot;</ETag><Size>{}</Size><StorageClass>STANDARD</StorageClass></Contents>",
                xml_escape(&object.key),
                xml_escape(&object.created_at),
                xml_escape(&object.sha256),
                object.size_bytes
            )
        })
        .collect::<String>();
    let common_prefixes = page
        .common_prefixes
        .iter()
        .map(|prefix| format!("<CommonPrefixes><Prefix>{}</Prefix></CommonPrefixes>", xml_escape(prefix)))
        .collect::<String>();
    let delimiter = query
        .delimiter
        .as_deref()
        .map(|value| format!("<Delimiter>{}</Delimiter>", xml_escape(value)))
        .unwrap_or_default();
    let continuation_token = query
        .continuation_token
        .as_deref()
        .map(|value| format!("<ContinuationToken>{}</ContinuationToken>", xml_escape(value)))
        .unwrap_or_default();
    let next_continuation_token = page
        .next_continuation_token
        .as_deref()
        .map(|value| format!("<NextContinuationToken>{}</NextContinuationToken>", xml_escape(value)))
        .unwrap_or_default();
    let start_after = page
        .start_after
        .as_deref()
        .map(|value| format!("<StartAfter>{}</StartAfter>", xml_escape(value)))
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
        <Name>{}</Name><Prefix>{}</Prefix>{}<KeyCount>{}</KeyCount><MaxKeys>{}</MaxKeys>\
        <IsTruncated>{}</IsTruncated>{}{}{}{}{}</ListBucketResult>",
        xml_escape(bucket_name),
        xml_escape(prefix),
        delimiter,
        page.key_count,
        page.max_keys,
        if page.is_truncated { "true" } else { "false" },
        continuation_token,
        next_continuation_token,
        start_after,
        contents,
        common_prefixes
    )
}

fn bucket_location_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<LocationConstraint xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">us-east-1</LocationConstraint>"
        .to_owned()
}

fn copy_object_xml(object: &ObjectSummary) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<CopyObjectResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<LastModified>{}</LastModified><ETag>&quot;{}&quot;</ETag></CopyObjectResult>",
        xml_escape(&object.created_at),
        xml_escape(&object.sha256)
    )
}

fn delete_objects_xml(result: &DeleteObjectsResult) -> String {
    let deleted = result
        .deleted
        .iter()
        .map(|key| format!("<Deleted><Key>{}</Key></Deleted>", xml_escape(key)))
        .collect::<String>();
    let errors = result
        .errors
        .iter()
        .map(|error| {
            format!(
                "<Error><Key>{}</Key><Code>{}</Code><Message>{}</Message></Error>",
                xml_escape(&error.key),
                xml_escape(&error.code),
                xml_escape(&error.message)
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<DeleteResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{}{}</DeleteResult>",
        deleted, errors
    )
}

fn initiate_multipart_upload_xml(upload: &MultipartUploadRecord) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<InitiateMultipartUploadResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Bucket>{}</Bucket><Key>{}</Key><UploadId>{}</UploadId></InitiateMultipartUploadResult>",
        xml_escape(&upload.bucket_name),
        xml_escape(&upload.object_key),
        xml_escape(&upload.upload_id)
    )
}

fn list_multipart_uploads_xml(bucket_name: &str, uploads: &[MultipartUploadRecord]) -> String {
    let uploads_xml = uploads
        .iter()
        .map(|upload| {
            format!(
                "<Upload><Key>{}</Key><UploadId>{}</UploadId>\
<Initiator><ID>{}</ID><DisplayName>{}</DisplayName></Initiator>\
<Owner><ID>pontemesh</ID><DisplayName>Ponte Mesh</DisplayName></Owner>\
<StorageClass>STANDARD</StorageClass><Initiated>{}</Initiated></Upload>",
                xml_escape(&upload.object_key),
                xml_escape(&upload.upload_id),
                xml_escape("s3"),
                xml_escape("s3"),
                xml_escape(&upload.initiated_at)
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<ListMultipartUploadsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Bucket>{}</Bucket><KeyMarker></KeyMarker><UploadIdMarker></UploadIdMarker>\
<NextKeyMarker></NextKeyMarker><NextUploadIdMarker></NextUploadIdMarker>\
<MaxUploads>1000</MaxUploads><IsTruncated>false</IsTruncated>{}</ListMultipartUploadsResult>",
        xml_escape(bucket_name),
        uploads_xml
    )
}

fn list_parts_xml(
    bucket_name: &str,
    object_key: &str,
    upload_id: &str,
    parts: &[MultipartPartRecord],
) -> String {
    let parts_xml = parts
        .iter()
        .map(|part| {
            format!(
                "<Part><PartNumber>{}</PartNumber><LastModified>{}</LastModified>\
<ETag>&quot;{}&quot;</ETag><Size>{}</Size></Part>",
                part.part_number,
                xml_escape(&part.uploaded_at),
                xml_escape(&part.etag),
                part.size_bytes
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<ListPartsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Bucket>{}</Bucket><Key>{}</Key><UploadId>{}</UploadId>\
<StorageClass>STANDARD</StorageClass><PartNumberMarker>0</PartNumberMarker>\
<NextPartNumberMarker>0</NextPartNumberMarker><MaxParts>1000</MaxParts>\
<IsTruncated>false</IsTruncated>{}</ListPartsResult>",
        xml_escape(bucket_name),
        xml_escape(object_key),
        xml_escape(upload_id),
        parts_xml
    )
}

fn complete_multipart_upload_xml(bucket_name: &str, object_key: &str, etag: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<CompleteMultipartUploadResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Location>/{}/{}</Location><Bucket>{}</Bucket><Key>{}</Key><ETag>&quot;{}&quot;</ETag>\
</CompleteMultipartUploadResult>",
        xml_escape(bucket_name),
        xml_escape(object_key),
        xml_escape(bucket_name),
        xml_escape(object_key),
        xml_escape(etag)
    )
}

fn bucket_versioning_xml(enabled: bool) -> String {
    let status = if enabled {
        "<Status>Enabled</Status>"
    } else {
        "<Status>Suspended</Status>"
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <VersioningConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{status}</VersioningConfiguration>"
    )
}

fn object_tagging_xml(tags: &[S3ObjectTag]) -> String {
    let tags_xml = tags
        .iter()
        .map(|tag| {
            format!(
                "<Tag><Key>{}</Key><Value>{}</Value></Tag>",
                xml_escape(&tag.key),
                xml_escape(&tag.value)
            )
        })
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <Tagging xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><TagSet>{tags_xml}</TagSet></Tagging>"
    )
}

fn multipart_upload_dir(state: &AppState, upload_id: &str) -> anyhow::Result<PathBuf> {
    Ok(config::configured_storage_dir(&state.paths)?
        .join("multipart")
        .join(upload_id))
}

fn remove_multipart_part_files(parts: &[MultipartPartRecord]) {
    for part in parts {
        let _ = fs::remove_file(&part.storage_path);
    }
}

fn request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn object_tagging_enabled(state: &AppState, bucket_name: &str) -> anyhow::Result<bool> {
    Ok(state
        .catalog
        .get_bucket_policy(bucket_name)
        .await?
        .s3_object_tagging_enabled)
}

fn tagging_disabled_response(bucket_name: &str, object_key: &str) -> Response {
    s3_error(
        StatusCode::BAD_REQUEST,
        "NotImplemented",
        "Object tagging is disabled for this bucket",
        Some(bucket_name),
        Some(object_key),
    )
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_owned())
}

fn extract_xml_blocks(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut rest = xml;
    let mut blocks = Vec::new();
    while let Some(open_index) = rest.find(&open) {
        let content_start = open_index + open.len();
        let Some(close_index) = rest[content_start..].find(&close) else {
            break;
        };
        let content_end = content_start + close_index;
        blocks.push(rest[content_start..content_end].to_owned());
        rest = &rest[(content_end + close.len())..];
    }
    blocks
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_unescape(raw: &str) -> String {
    raw.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn percent_decode(raw: &str) -> String {
    let mut output = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&raw[index + 1..index + 3], 16) {
                output.push(value);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

async fn record_origin_audit(
    state: &AppState,
    event: &str,
    principal: &str,
    outcome: &str,
    detail: &str,
) {
    if let Err(error) = state
        .catalog
        .record_audit_event(event, Some(principal), outcome, detail)
        .await
    {
        audit::failure("audit_persist_failed", Some(principal), &error.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn list_buckets_xml_uses_s3_result_shape_and_escapes_names() {
        let xml = list_buckets_xml(&[
            BucketSummary {
                name: "media-bucket".to_owned(),
                object_count: 2,
                total_bytes: 42,
                created_at: "2026-06-29T12:00:00Z".to_owned(),
            },
            BucketSummary {
                name: "team&docs".to_owned(),
                object_count: 1,
                total_bytes: 7,
                created_at: "2026-06-29T12:01:00Z".to_owned(),
            },
        ]);

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains(
            "<ListAllMyBucketsResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">"
        ));
        assert!(
            xml.contains("<Owner><ID>pontemesh</ID><DisplayName>Ponte Mesh</DisplayName></Owner>")
        );
        assert!(xml.contains("<Bucket><Name>media-bucket</Name><CreationDate>2026-06-29T12:00:00Z</CreationDate></Bucket>"));
        assert!(xml.contains("<Bucket><Name>team&amp;docs</Name><CreationDate>2026-06-29T12:01:00Z</CreationDate></Bucket>"));
    }

    #[test]
    fn list_objects_v2_xml_filters_by_prefix_and_uses_s3_fields() {
        let query = ListObjectsQuery {
            list_type: Some("2".to_owned()),
            prefix: Some("photos/".to_owned()),
            delimiter: Some("/".to_owned()),
            max_keys: Some(1),
            continuation_token: None,
            start_after: Some("photos/a.jpg".to_owned()),
            uploads: None,
            location: None,
            delete: None,
            versioning: None,
        };
        let page = S3ListObjectsPage {
            items: vec![ObjectSummary {
                key: "photos/cat & dog.jpg".to_owned(),
                size_bytes: 123,
                content_type: "image/jpeg".to_owned(),
                sha256: "abc123".to_owned(),
                created_at: "2026-06-29T12:00:00Z".to_owned(),
                updated_at: "2026-06-29T12:00:00Z".to_owned(),
                state: "AVAILABLE".to_owned(),
            }],
            common_prefixes: vec!["photos/nested/".to_owned()],
            key_count: 2,
            max_keys: 2,
            is_truncated: true,
            next_continuation_token: Some("photos/nested/file.txt".to_owned()),
            start_after: Some("photos/a.jpg".to_owned()),
        };
        let xml = list_objects_v2_xml(
            "media-bucket",
            &query,
            &page,
        );

        assert!(
            xml.contains("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">")
        );
        assert!(xml.contains("<Name>media-bucket</Name>"));
        assert!(xml.contains("<Prefix>photos/</Prefix>"));
        assert!(xml.contains("<Delimiter>/</Delimiter>"));
        assert!(xml.contains("<KeyCount>2</KeyCount>"));
        assert!(xml.contains("<MaxKeys>2</MaxKeys>"));
        assert!(xml.contains("<IsTruncated>true</IsTruncated>"));
        assert!(xml.contains("<NextContinuationToken>photos/nested/file.txt</NextContinuationToken>"));
        assert!(xml.contains("<StartAfter>photos/a.jpg</StartAfter>"));
        assert!(xml.contains("<Key>photos/cat &amp; dog.jpg</Key>"));
        assert!(xml.contains("<CommonPrefixes><Prefix>photos/nested/</Prefix></CommonPrefixes>"));
        assert!(xml.contains("<ETag>&quot;abc123&quot;</ETag>"));
        assert!(xml.contains("<Size>123</Size>"));
        assert!(xml.contains("<StorageClass>STANDARD</StorageClass>"));
    }

    #[test]
    fn bucket_versioning_and_object_tagging_xml_use_s3_shapes() {
        let enabled = bucket_versioning_xml(true);
        assert!(enabled.contains("<VersioningConfiguration"));
        assert!(enabled.contains("<Status>Enabled</Status>"));

        let tags = object_tagging_xml(&[
            S3ObjectTag {
                key: "project".to_owned(),
                value: "ponte&mesh".to_owned(),
            },
            S3ObjectTag {
                key: "env".to_owned(),
                value: "dev".to_owned(),
            },
        ]);
        assert!(tags.contains("<Tagging"));
        assert!(tags.contains("<Key>project</Key><Value>ponte&amp;mesh</Value>"));
        assert!(tags.contains("<Key>env</Key><Value>dev</Value>"));
    }

    #[tokio::test]
    async fn s3_error_response_is_xml_and_includes_context_fields() {
        let response = s3_error(
            StatusCode::NOT_FOUND,
            "NoSuchKey",
            "The specified key does not exist",
            Some("media-bucket"),
            Some("photos/missing&file.txt"),
        );

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .expect("content type")
                .to_str()
                .expect("content type text"),
            "application/xml"
        );
        assert!(response.headers().contains_key("x-amz-request-id"));
        let body = response_text(response).await;
        assert!(body.contains("<Error><Code>NoSuchKey</Code>"));
        assert!(body.contains("<BucketName>media-bucket</BucketName>"));
        assert!(body.contains("<Key>photos/missing&amp;file.txt</Key>"));
        assert!(body.contains("<RequestId>"));
    }

    #[tokio::test]
    async fn object_metadata_response_sets_s3_headers_without_body() {
        let object = object_record();
        let response = object_metadata_response(&object, true);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header_text(&response, header::CONTENT_TYPE), "text/plain");
        assert_eq!(header_text(&response, header::CONTENT_LENGTH), "11");
        assert_eq!(header_text(&response, header::ACCEPT_RANGES), "bytes");
        assert_eq!(
            header_text(&response, "ETag"),
            "\"64ec88ca00b268e5ba1a35678a1b5316d212f4f366b2477232534a8aeca37f3c\""
        );
        assert_eq!(
            header_text(&response, "Last-Modified"),
            "2026-06-29T12:00:00Z"
        );
        assert_eq!(header_text(&response, "x-amz-bucket-region"), "us-east-1");
        assert_eq!(
            header_text(&response, "x-pontemesh-object-state"),
            "AVAILABLE"
        );
        assert_eq!(
            header_text(&response, "x-pontemesh-object-key"),
            "folder/hello.txt"
        );
        assert!(response_text(response).await.is_empty());
    }

    #[tokio::test]
    async fn object_body_response_sets_full_and_partial_object_headers() {
        let object = object_record();

        let full = object_body_response(&object, StatusCode::OK, b"hello world".to_vec(), None);
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(header_text(&full, header::CONTENT_LENGTH), "11");
        assert!(full.headers().get(header::CONTENT_RANGE).is_none());
        assert_eq!(response_text(full).await, "hello world");

        let partial = object_body_response(
            &object,
            StatusCode::PARTIAL_CONTENT,
            b"hello".to_vec(),
            Some(ResolvedRange { start: 0, end: 4 }),
        );
        assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(header_text(&partial, header::CONTENT_LENGTH), "5");
        assert_eq!(header_text(&partial, header::CONTENT_RANGE), "bytes 0-4/11");
        assert_eq!(response_text(partial).await, "hello");
    }

    #[test]
    fn parse_range_supports_standard_open_and_suffix_ranges() {
        let first = parse_range("bytes=0-4", 11).expect("first range");
        assert_eq!((first.start, first.end), (0, 4));

        let open = parse_range("bytes=6-", 11).expect("open range");
        assert_eq!((open.start, open.end), (6, 10));

        let suffix = parse_range("bytes=-5", 11).expect("suffix range");
        assert_eq!((suffix.start, suffix.end), (6, 10));
    }

    #[test]
    fn parse_range_rejects_invalid_s3_ranges() {
        assert!(parse_range("items=0-4", 11).is_err());
        assert!(parse_range("bytes=4-0", 11).is_err());
        assert!(parse_range("bytes=0-99", 11).is_err());
        assert!(parse_range("bytes=0-1,3-4", 11).is_err());
        assert!(parse_range("bytes=-0", 11).is_err());
    }

    fn object_record() -> ObjectRecord {
        ObjectRecord {
            key: "folder/hello.txt".to_owned(),
            size_bytes: 11,
            content_type: "text/plain".to_owned(),
            sha256: "64ec88ca00b268e5ba1a35678a1b5316d212f4f366b2477232534a8aeca37f3c".to_owned(),
            storage_path: "/tmp/pontemesh-test-object".to_owned(),
            created_at: "2026-06-29T12:00:00Z".to_owned(),
            state: "AVAILABLE".to_owned(),
        }
    }

    fn header_text<K>(response: &Response, name: K) -> &str
    where
        K: axum::http::header::AsHeaderName,
    {
        response
            .headers()
            .get(name)
            .expect("header present")
            .to_str()
            .expect("header text")
    }

    async fn response_text(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body");
        String::from_utf8(bytes.to_vec()).expect("utf8 response")
    }
}
