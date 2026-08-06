CREATE TABLE replica_health_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    replica_id UUID NOT NULL REFERENCES replica_credentials(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    version TEXT NULL,
    storage_available_bytes BIGINT NULL,
    error_count BIGINT NOT NULL DEFAULT 0,
    detail JSONB NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_replica_health_reports_replica_id_reported_at
    ON replica_health_reports(replica_id, reported_at DESC);

CREATE TABLE replica_metric_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    replica_id UUID NOT NULL REFERENCES replica_credentials(id) ON DELETE CASCADE,
    bytes_synced BIGINT NOT NULL DEFAULT 0,
    bytes_served BIGINT NOT NULL DEFAULT 0,
    fragments_synced BIGINT NOT NULL DEFAULT 0,
    fragments_served BIGINT NOT NULL DEFAULT 0,
    sync_failures BIGINT NOT NULL DEFAULT 0,
    auth_failures BIGINT NOT NULL DEFAULT 0,
    avg_latency_ms BIGINT NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_replica_metric_events_replica_id_reported_at
    ON replica_metric_events(replica_id, reported_at DESC);

CREATE TABLE replica_policy_updates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    replica_id UUID NOT NULL REFERENCES replica_credentials(id) ON DELETE CASCADE,
    update_type TEXT NOT NULL,
    bucket_id UUID NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_id UUID NULL REFERENCES objects(id) ON DELETE CASCADE,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_replica_policy_updates_replica_id_created_at
    ON replica_policy_updates(replica_id, created_at DESC);
