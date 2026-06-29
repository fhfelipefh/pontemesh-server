CREATE TABLE object_manifests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_version_id UUID NOT NULL UNIQUE REFERENCES object_versions(id) ON DELETE CASCADE,
    fragment_size_bytes BIGINT NOT NULL,
    object_hash_algorithm TEXT NOT NULL,
    object_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_object_manifests_object_version_id ON object_manifests(object_version_id);

CREATE TABLE object_manifest_fragments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    manifest_id UUID NOT NULL REFERENCES object_manifests(id) ON DELETE CASCADE,
    fragment_index BIGINT NOT NULL,
    byte_range_start BIGINT NOT NULL,
    byte_range_end BIGINT NOT NULL,
    size_bytes BIGINT NOT NULL,
    hash_algorithm TEXT NOT NULL,
    fragment_hash TEXT NOT NULL,
    priority TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(manifest_id, fragment_index)
);

CREATE INDEX idx_object_manifest_fragments_manifest_id ON object_manifest_fragments(manifest_id);

ALTER TABLE access_packages
    ADD COLUMN object_manifest_id UUID NULL REFERENCES object_manifests(id) ON DELETE RESTRICT;

CREATE INDEX idx_access_packages_object_manifest_id ON access_packages(object_manifest_id);
