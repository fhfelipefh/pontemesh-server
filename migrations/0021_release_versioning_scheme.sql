ALTER TABLE bucket_policies
    ADD COLUMN release_versioning_scheme TEXT NOT NULL DEFAULT 'DISABLED';

ALTER TABLE bucket_policy_defaults
    ADD COLUMN release_versioning_scheme TEXT NOT NULL DEFAULT 'DISABLED';

ALTER TABLE bucket_policies
    ADD CONSTRAINT chk_bucket_policies_release_versioning_scheme
    CHECK (release_versioning_scheme IN ('DISABLED', 'SEMVER', 'BUILD_NUMBER', 'CHANNEL', 'TAG'));

ALTER TABLE bucket_policy_defaults
    ADD CONSTRAINT chk_bucket_policy_defaults_release_versioning_scheme
    CHECK (release_versioning_scheme IN ('DISABLED', 'SEMVER', 'BUILD_NUMBER', 'CHANNEL', 'TAG'));
