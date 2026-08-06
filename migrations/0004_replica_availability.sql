CREATE TABLE replica_object_availability (
    replica_id UUID NOT NULL REFERENCES replica_credentials(id) ON DELETE CASCADE,
    bucket_id UUID NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    object_id UUID NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    object_manifest_id UUID NOT NULL REFERENCES object_manifests(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    available_fragments JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (replica_id, object_manifest_id)
);

CREATE INDEX idx_replica_object_availability_object_manifest_id
    ON replica_object_availability(object_manifest_id);

CREATE INDEX idx_replica_object_availability_last_seen_at
    ON replica_object_availability(last_seen_at);
