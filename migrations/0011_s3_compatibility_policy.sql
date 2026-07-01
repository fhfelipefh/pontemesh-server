ALTER TABLE bucket_policies
    ADD COLUMN s3_list_default_max_keys BIGINT NOT NULL DEFAULT 1000,
    ADD COLUMN s3_list_max_keys_limit BIGINT NOT NULL DEFAULT 10000,
    ADD COLUMN s3_list_allow_delimiter BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN s3_versioning_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN s3_object_tagging_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN s3_checksum_algorithm TEXT NOT NULL DEFAULT 'SHA256',
    ADD COLUMN s3_multipart_abort_days BIGINT NOT NULL DEFAULT 7;

CREATE TABLE s3_object_tags (
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    tag_key TEXT NOT NULL,
    tag_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (object_id, tag_key)
);

CREATE INDEX idx_s3_object_tags_object_id ON s3_object_tags(object_id);
