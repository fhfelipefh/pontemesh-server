ALTER TABLE bucket_policies
    ADD COLUMN source_selection_strategy TEXT NOT NULL DEFAULT 'ORIGIN_REPLICA_EDGE',
    ADD COLUMN fragment_priority_strategy TEXT NOT NULL DEFAULT 'MANIFEST_ORDER',
    ADD COLUMN failure_threshold BIGINT NOT NULL DEFAULT 3,
    ADD COLUMN fallback_mode TEXT NOT NULL DEFAULT 'ORIGIN_RANGE';

CREATE TABLE peer_fragment_availability (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    access_package_id UUID NOT NULL REFERENCES access_packages(id) ON DELETE CASCADE,
    application_id UUID NOT NULL REFERENCES application_credentials(id) ON DELETE CASCADE,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    object_manifest_id UUID NOT NULL REFERENCES object_manifests(id) ON DELETE CASCADE,
    peer_id TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    available_fragments JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    announced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(access_package_id, peer_id, object_manifest_id)
);

CREATE INDEX idx_peer_fragment_availability_manifest_expires
    ON peer_fragment_availability(object_manifest_id, expires_at);

CREATE INDEX idx_peer_fragment_availability_package
    ON peer_fragment_availability(access_package_id);

ALTER TABLE fragment_transfer_events
    ADD COLUMN access_package_id UUID NULL REFERENCES access_packages(id) ON DELETE SET NULL,
    ADD COLUMN peer_id UUID NULL REFERENCES peer_fragment_availability(id) ON DELETE SET NULL,
    ADD COLUMN latency_ms BIGINT NULL;

CREATE INDEX idx_fragment_transfer_events_access_package_created_at
    ON fragment_transfer_events(access_package_id, created_at DESC);

CREATE INDEX idx_fragment_transfer_events_event_outcome_created_at
    ON fragment_transfer_events(event_type, outcome, created_at DESC);
