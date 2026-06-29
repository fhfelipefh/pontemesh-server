CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ NULL
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL,
    user_agent TEXT NULL,
    ip_address INET NULL
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_sessions_revoked_at ON sessions(revoked_at);

CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    actor_user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    ip_address INET NULL,
    user_agent TEXT NULL,
    metadata JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_events_event_type ON audit_events(event_type);
CREATE INDEX idx_audit_events_actor_user_id ON audit_events(actor_user_id);
CREATE INDEX idx_audit_events_created_at ON audit_events(created_at);

CREATE TABLE buckets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL
);

CREATE TABLE objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE RESTRICT,
    object_key TEXT NOT NULL,
    current_version_id UUID NULL,
    state TEXT NOT NULL DEFAULT 'AVAILABLE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    UNIQUE(bucket_id, object_key)
);

CREATE INDEX idx_objects_bucket_id ON objects(bucket_id);
CREATE INDEX idx_objects_state ON objects(state);
CREATE INDEX idx_objects_deleted_at ON objects(deleted_at);

CREATE TABLE object_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    version_number BIGINT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_type TEXT NULL,
    hash_algorithm TEXT NOT NULL,
    object_hash TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(object_id, version_number)
);

CREATE INDEX idx_object_versions_object_id ON object_versions(object_id);

ALTER TABLE objects
    ADD CONSTRAINT fk_objects_current_version
    FOREIGN KEY (current_version_id) REFERENCES object_versions(id)
    ON DELETE SET NULL;

CREATE TABLE bucket_policies (
    bucket_id UUID PRIMARY KEY REFERENCES buckets(id) ON DELETE CASCADE,
    access_package_ttl_seconds BIGINT NOT NULL DEFAULT 900,
    fragment_size_bytes BIGINT NOT NULL DEFAULT 4194304,
    allow_replica_edge BOOLEAN NOT NULL DEFAULT FALSE,
    allow_peer_sharing BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE application_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scopes JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ NULL
);

CREATE TABLE s3_access_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    access_key_id TEXT NOT NULL UNIQUE,
    secret_key_hash TEXT NOT NULL,
    secret_key_ciphertext BYTEA NULL,
    user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ NULL,
    last_used_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_s3_access_keys_user_id ON s3_access_keys(user_id);
CREATE INDEX idx_s3_access_keys_active ON s3_access_keys(is_active);

CREATE TABLE replica_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    allowed_buckets JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ NULL
);

CREATE TABLE access_packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_token_hash TEXT NOT NULL UNIQUE,
    application_id UUID NOT NULL REFERENCES application_credentials(id) ON DELETE RESTRICT,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE RESTRICT,
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE RESTRICT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_access_packages_application_id ON access_packages(application_id);
CREATE INDEX idx_access_packages_expires_at ON access_packages(expires_at);
CREATE INDEX idx_access_packages_revoked_at ON access_packages(revoked_at);

CREATE TABLE origin_transfer_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID NOT NULL REFERENCES application_credentials(id) ON DELETE RESTRICT,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE RESTRICT,
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE RESTRICT,
    bytes_served BIGINT NOT NULL,
    range_start BIGINT NULL,
    range_end BIGINT NULL,
    status_code INTEGER NOT NULL,
    served_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_origin_transfer_events_served_at ON origin_transfer_events(served_at);
CREATE INDEX idx_origin_transfer_events_bucket_id ON origin_transfer_events(bucket_id);
CREATE INDEX idx_origin_transfer_events_object_id ON origin_transfer_events(object_id);
