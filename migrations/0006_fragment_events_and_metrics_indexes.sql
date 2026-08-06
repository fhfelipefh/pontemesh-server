CREATE TABLE fragment_transfer_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_type TEXT NOT NULL,
    replica_id UUID NULL REFERENCES replica_credentials(id) ON DELETE SET NULL,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    object_manifest_id UUID NOT NULL REFERENCES object_manifests(id) ON DELETE CASCADE,
    fragment_index BIGINT NOT NULL,
    fragment_hash TEXT NOT NULL,
    event_type TEXT NOT NULL,
    bytes_transferred BIGINT NOT NULL DEFAULT 0,
    outcome TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_fragment_transfer_events_manifest_fragment
    ON fragment_transfer_events(object_manifest_id, fragment_index, created_at DESC);

CREATE INDEX idx_fragment_transfer_events_replica_created_at
    ON fragment_transfer_events(replica_id, created_at DESC);
