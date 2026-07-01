CREATE TABLE s3_multipart_uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_key TEXT NOT NULL,
    content_type TEXT NOT NULL,
    initiated_by TEXT NOT NULL,
    initiated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ NULL,
    aborted_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_s3_multipart_uploads_bucket_active
    ON s3_multipart_uploads(bucket_id, initiated_at DESC)
    WHERE completed_at IS NULL AND aborted_at IS NULL;

CREATE TABLE s3_multipart_upload_parts (
    upload_id UUID NOT NULL REFERENCES s3_multipart_uploads(id) ON DELETE CASCADE,
    part_number INTEGER NOT NULL,
    etag TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (upload_id, part_number)
);

CREATE INDEX idx_s3_multipart_upload_parts_upload
    ON s3_multipart_upload_parts(upload_id, part_number);
