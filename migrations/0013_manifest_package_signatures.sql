ALTER TABLE object_manifests
    ADD COLUMN signature_algorithm TEXT NULL,
    ADD COLUMN signature TEXT NULL;

ALTER TABLE access_packages
    ADD COLUMN signature_algorithm TEXT NULL,
    ADD COLUMN signature TEXT NULL;
