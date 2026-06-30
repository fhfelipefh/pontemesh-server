use crate::{
    audit,
    catalog::{
        self, BucketSummary, NewObject, NewObjectFragment, NewObjectManifest, ObjectRecord,
        ObjectSummary,
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
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    #[serde(rename = "list-type")]
    list_type: Option<String>,
    prefix: Option<String>,
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

pub async fn create_bucket(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    Path(bucket_name): Path<String>,
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
    if query.list_type.as_deref() != Some("2") {
        return s3_error(
            StatusCode::BAD_REQUEST,
            "InvalidArgument",
            "Only ListObjectsV2 is supported",
            Some(&bucket_name),
            None,
        );
    }

    match state.catalog.list_objects(&bucket_name).await {
        Ok(objects) => s3_xml_response(
            StatusCode::OK,
            list_objects_v2_xml(&bucket_name, query.prefix.as_deref(), &objects),
        ),
        Err(error) => s3_bad_request(error, Some(&bucket_name), None),
    }
}

pub async fn put_object(
    State(state): State<AppState>,
    Extension(identity): Extension<S3Identity>,
    headers: HeaderMap,
    Path((bucket_name, object_key)): Path<(String, String)>,
    body: Body,
) -> Response {
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
) -> Response {
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
    Path((bucket_name, object_key)): Path<(String, String)>,
) -> Response {
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

struct StreamedObject {
    size_bytes: i64,
    sha256: String,
    manifest: NewObjectManifest,
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

fn list_objects_v2_xml(
    bucket_name: &str,
    prefix: Option<&str>,
    objects: &[ObjectSummary],
) -> String {
    let prefix = prefix.unwrap_or("");
    let filtered: Vec<&ObjectSummary> = objects
        .iter()
        .filter(|object| object.key.starts_with(prefix))
        .collect();
    let contents = filtered
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
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
<Name>{}</Name><Prefix>{}</Prefix><KeyCount>{}</KeyCount><MaxKeys>1000</MaxKeys>\
<IsTruncated>false</IsTruncated>{}</ListBucketResult>",
        xml_escape(bucket_name),
        xml_escape(prefix),
        filtered.len(),
        contents
    )
}

fn request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
        let xml = list_objects_v2_xml(
            "media-bucket",
            Some("photos/"),
            &[
                ObjectSummary {
                    key: "photos/cat & dog.jpg".to_owned(),
                    size_bytes: 123,
                    content_type: "image/jpeg".to_owned(),
                    sha256: "abc123".to_owned(),
                    created_at: "2026-06-29T12:00:00Z".to_owned(),
                    state: "AVAILABLE".to_owned(),
                },
                ObjectSummary {
                    key: "videos/clip.mp4".to_owned(),
                    size_bytes: 456,
                    content_type: "video/mp4".to_owned(),
                    sha256: "def456".to_owned(),
                    created_at: "2026-06-29T12:02:00Z".to_owned(),
                    state: "AVAILABLE".to_owned(),
                },
            ],
        );

        assert!(
            xml.contains("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">")
        );
        assert!(xml.contains("<Name>media-bucket</Name>"));
        assert!(xml.contains("<Prefix>photos/</Prefix>"));
        assert!(xml.contains("<KeyCount>1</KeyCount>"));
        assert!(xml.contains("<IsTruncated>false</IsTruncated>"));
        assert!(xml.contains("<Key>photos/cat &amp; dog.jpg</Key>"));
        assert!(xml.contains("<ETag>&quot;abc123&quot;</ETag>"));
        assert!(xml.contains("<Size>123</Size>"));
        assert!(xml.contains("<StorageClass>STANDARD</StorageClass>"));
        assert!(!xml.contains("videos/clip.mp4"));
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
