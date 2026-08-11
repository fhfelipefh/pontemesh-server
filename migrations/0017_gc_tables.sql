CREATE TABLE gc_cycles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    epoch BIGINT NOT NULL UNIQUE,
    state TEXT NOT NULL DEFAULT 'MARKING',
    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    mark_finished_at TIMESTAMPTZ NULL,
    sweep_finished_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    is_dry_run BOOLEAN NOT NULL DEFAULT FALSE,
    candidates BIGINT NOT NULL DEFAULT 0,
    objects_reclaimed BIGINT NOT NULL DEFAULT 0,
    bytes_reclaimed BIGINT NOT NULL DEFAULT 0,
    errors BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_gc_cycles_epoch ON gc_cycles(epoch DESC);
CREATE INDEX idx_gc_cycles_state ON gc_cycles(state);

CREATE TABLE gc_candidates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_type TEXT NOT NULL,
    resource_id UUID NULL,
    storage_path TEXT NULL,
    reason TEXT NOT NULL,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    not_before TIMESTAMPTZ NOT NULL,
    state TEXT NOT NULL DEFAULT 'PENDING',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ NULL,
    last_error TEXT NULL,
    sweep_token UUID NULL,
    sweep_lease_until TIMESTAMPTZ NULL,
    quarantine_path TEXT NULL,
    quarantined_at TIMESTAMPTZ NULL,
    deleted_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_gc_candidates_state_not_before ON gc_candidates(state, not_before)
    WHERE deleted_at IS NULL;
CREATE INDEX idx_gc_candidates_resource ON gc_candidates(resource_type, resource_id)
    WHERE deleted_at IS NULL;
CREATE INDEX idx_gc_candidates_storage_path ON gc_candidates(storage_path)
    WHERE storage_path IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX idx_gc_candidates_sweep_lease ON gc_candidates(sweep_lease_until)
    WHERE state = 'SWEEPING';
