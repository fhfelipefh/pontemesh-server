CREATE TABLE replica_request_nonces (
    replica_id UUID NOT NULL REFERENCES replica_credentials(id) ON DELETE CASCADE,
    nonce TEXT NOT NULL,
    seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (replica_id, nonce)
);

CREATE INDEX idx_replica_request_nonces_seen_at ON replica_request_nonces(seen_at);
