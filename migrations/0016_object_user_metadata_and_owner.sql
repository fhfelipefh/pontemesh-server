ALTER TABLE object_versions
    ADD COLUMN user_metadata JSONB NULL,
    ADD COLUMN created_by TEXT NULL;

ALTER TABLE s3_multipart_uploads
    ADD COLUMN user_metadata JSONB NULL;
