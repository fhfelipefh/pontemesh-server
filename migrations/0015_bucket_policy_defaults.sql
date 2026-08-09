CREATE TABLE bucket_policy_defaults (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    access_package_ttl_seconds BIGINT NOT NULL DEFAULT 900,
    fragment_size_bytes BIGINT NOT NULL DEFAULT 4194304,
    allow_replica_edge BOOLEAN NOT NULL DEFAULT FALSE,
    allow_peer_sharing BOOLEAN NOT NULL DEFAULT FALSE,
    source_selection_strategy TEXT NOT NULL DEFAULT 'ORIGIN_REPLICA_EDGE',
    fragment_priority_strategy TEXT NOT NULL DEFAULT 'MANIFEST_ORDER',
    failure_threshold BIGINT NOT NULL DEFAULT 3,
    fallback_mode TEXT NOT NULL DEFAULT 'ORIGIN_RANGE',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO bucket_policy_defaults (singleton)
VALUES (TRUE)
ON CONFLICT (singleton) DO NOTHING;
