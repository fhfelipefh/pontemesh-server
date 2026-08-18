use crate::admin::ErrorResponse;
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Query, Request},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

const DEFAULT_DOWNLOAD_SIZE: usize = 16 * 1024 * 1024;
const MIN_SPEED_TEST_SIZE: usize = 1 * 1024 * 1024;
const MAX_DOWNLOAD_SIZE: usize = 256 * 1024 * 1024;
const MAX_UPLOAD_SIZE: usize = 512 * 1024 * 1024;
const STREAM_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadQuery {
    size: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    bytes_received: u64,
}

pub async fn download(Query(query): Query<DownloadQuery>) -> Response {
    let size = query
        .size
        .unwrap_or(DEFAULT_DOWNLOAD_SIZE)
        .clamp(MIN_SPEED_TEST_SIZE, MAX_DOWNLOAD_SIZE);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, size.to_string())
        .header(
            header::CACHE_CONTROL,
            "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0",
        )
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .header(header::CONTENT_ENCODING, "identity")
        .body(Body::from_stream(pseudo_random_stream(size)))
        .expect("valid speed test download response")
}

pub async fn upload(request: Request) -> Response {
    let mut bytes_received: u64 = 0;
    let mut stream = request.into_body().into_data_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                bytes_received = match bytes_received.checked_add(bytes.len() as u64) {
                    Some(total) if total <= MAX_UPLOAD_SIZE as u64 => total,
                    _ => {
                        return (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            Json(ErrorResponse {
                                error: format!(
                                    "speed test upload exceeds the {MAX_UPLOAD_SIZE} byte limit"
                                ),
                            }),
                        )
                            .into_response();
                    }
                };
            }
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("failed to read upload body: {error}"),
                    }),
                )
                    .into_response();
            }
        }
    }
    Json(UploadResult { bytes_received }).into_response()
}

fn pseudo_random_stream(
    size: usize,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static {
    futures_util::stream::unfold(
        (size, xorshift_seed()),
        |(remaining, mut state)| async move {
            if remaining == 0 {
                return None;
            }
            let chunk_len = remaining.min(STREAM_CHUNK_SIZE);
            let mut chunk = Vec::with_capacity(chunk_len);
            for _ in 0..chunk_len {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                chunk.push((state & 0xFF) as u8);
            }
            Some((
                Ok::<_, std::io::Error>(Bytes::from(chunk)),
                (remaining - chunk_len, state),
            ))
        },
    )
}

fn xorshift_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    nanos ^ (std::process::id() as u64) << 32 ^ 0x9E37_79B9_7F4A_7C15
}
