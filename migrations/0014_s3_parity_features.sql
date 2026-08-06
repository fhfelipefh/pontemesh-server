ALTER TABLE object_versions
    ADD COLUMN s3_version_id TEXT NULL,
    ADD COLUMN is_delete_marker BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN checksum_sha256 TEXT NULL,
    ADD COLUMN checksum_crc32 TEXT NULL,
    ADD COLUMN encryption_algorithm TEXT NULL,
    ADD COLUMN encryption_key_id TEXT NULL,
    ADD COLUMN encryption_nonce BYTEA NULL,
    ADD COLUMN object_lock_mode TEXT NULL,
    ADD COLUMN retain_until TIMESTAMPTZ NULL,
    ADD COLUMN legal_hold BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE object_versions
SET s3_version_id = id::text,
    checksum_sha256 = object_hash
WHERE s3_version_id IS NULL;

ALTER TABLE object_versions
    ALTER COLUMN s3_version_id SET NOT NULL;

CREATE UNIQUE INDEX idx_object_versions_s3_version_id ON object_versions(s3_version_id);
CREATE INDEX idx_object_versions_delete_marker ON object_versions(is_delete_marker);
CREATE INDEX idx_object_versions_retain_until ON object_versions(retain_until);

ALTER TABLE bucket_policies
    ADD COLUMN s3_default_encryption_algorithm TEXT NOT NULL DEFAULT 'NONE',
    ADD COLUMN s3_default_encryption_key_id TEXT NULL,
    ADD COLUMN s3_object_lock_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN s3_object_lock_default_mode TEXT NULL,
    ADD COLUMN s3_object_lock_default_retain_days BIGINT NULL,
    ADD COLUMN s3_lifecycle_rules JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN s3_resource_policy JSONB NOT NULL DEFAULT '{"Version":"2012-10-17","Statement":[]}'::jsonb,
    ADD COLUMN s3_event_notifications JSONB NOT NULL DEFAULT '{"EventBridgeEnabled":false,"Rules":[]}'::jsonb;

CREATE TABLE s3_notification_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_id UUID NULL REFERENCES objects(id) ON DELETE SET NULL,
    event_name TEXT NOT NULL,
    object_key TEXT NOT NULL,
    version_id TEXT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_s3_notification_events_bucket_created ON s3_notification_events(bucket_id, created_at);
CREATE INDEX idx_s3_notification_events_event_name ON s3_notification_events(event_name);
