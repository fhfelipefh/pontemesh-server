use anyhow::Context;
use axum::body::Body;
use futures_util::TryStreamExt;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

pub async fn file_body(path: &Path, range: Option<(u64, u64)>) -> anyhow::Result<(Body, u64, u64)> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("failed to open {}", path.display()))?;
    let total_size = file
        .metadata()
        .await
        .with_context(|| format!("failed to inspect {}", path.display()))?
        .len();
    let content_length = match range {
        Some((start, end)) => {
            file.seek(std::io::SeekFrom::Start(start)).await?;
            end.checked_sub(start)
                .and_then(|length| length.checked_add(1))
                .context("invalid file range")?
        }
        None => total_size,
    };
    let stream = ReaderStream::new(file.take(content_length));
    Ok((Body::from_stream(stream), total_size, content_length))
}

pub fn fragment_body(fragments: Vec<(std::path::PathBuf, u64)>, range: Option<(u64, u64)>) -> Body {
    let stream = async_stream::try_stream! {
        let mut object_offset = 0_u64;
        for (path, fragment_size) in fragments {
            let fragment_end = object_offset.saturating_add(fragment_size).saturating_sub(1);
            let requested_start = range.map(|value| value.0).unwrap_or(0);
            let requested_end = range.map(|value| value.1).unwrap_or(u64::MAX);
            if fragment_end < requested_start || object_offset > requested_end {
                object_offset = object_offset.saturating_add(fragment_size);
                continue;
            }
            let local_start = requested_start.saturating_sub(object_offset);
            let local_end = requested_end
                .min(fragment_end)
                .saturating_sub(object_offset);
            let mut file = tokio::fs::File::open(&path).await?;
            file.seek(std::io::SeekFrom::Start(local_start)).await?;
            let mut remaining = local_end.saturating_sub(local_start).saturating_add(1);
            let mut buffer = vec![0_u8; 64 * 1024];
            while remaining > 0 {
                let limit = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
                let read = file.read(&mut buffer[..limit]).await?;
                if read == 0 {
                    std::io::Result::<()>::Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "fragment ended early",
                    ))?;
                }
                remaining -= read as u64;
                yield axum::body::Bytes::copy_from_slice(&buffer[..read]);
            }
            object_offset = object_offset.saturating_add(fragment_size);
        }
    };
    Body::from_stream(stream.map_err(|error: std::io::Error| error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn file_and_fragment_streams_honor_ranges() {
        let root = std::env::temp_dir().join(format!(
            "pontemesh-streaming-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        tokio::fs::create_dir_all(&root).await.expect("test dir");
        let first = root.join("first");
        let second = root.join("second");
        tokio::fs::write(&first, b"hello").await.expect("first");
        tokio::fs::write(&second, b" world").await.expect("second");

        let (body, total, length) = file_body(&first, Some((1, 3))).await.expect("file body");
        assert_eq!((total, length), (5, 3));
        assert_eq!(body.collect().await.expect("body").to_bytes(), "ell");

        let body = fragment_body(vec![(first.clone(), 5), (second.clone(), 6)], Some((3, 8)));
        assert_eq!(body.collect().await.expect("body").to_bytes(), "lo wor");

        tokio::fs::remove_dir_all(root).await.expect("cleanup");
    }
}
