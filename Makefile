.PHONY: check i18n-check api-contract architecture-check migrations-check test-s3-parity test-s3-parity-embedded e2e-origin-replica e2e-ha-replica-election

check:
	$(MAKE) architecture-check
	$(MAKE) migrations-check
	$(MAKE) i18n-check
	cd web && npm run typecheck
	cd web && npm run lint
	cd web && npm run test:ui:quality
	$(MAKE) api-contract
	cargo check

i18n-check:
	cd web && npm run i18n:check

api-contract:
	cargo test api_contract

architecture-check:
	./scripts/check-architecture.sh

migrations-check:
	./scripts/check-migrations.sh

test-s3-parity:
	docker compose -p ponte-mesh -f docker/docker-compose.yml -f docker/docker-compose.test-postgres.yml up -d postgres
	TEST_DATABASE_URL=postgres://pontemesh:pontemesh@127.0.0.1:$${PONTEMESH_TEST_POSTGRES_PORT:-45432}/pontemesh cargo test s3_parity_features_cover_versioning_lifecycle_encryption_lock_policy_notifications_and_checksums -- --nocapture

test-s3-parity-embedded:
	cargo run --example s3_parity_embedded_postgres

e2e-origin-replica:
	./scripts/start-e2e-origin-replica.sh --reset
	cd web && npm run test:e2e:origin-replica

e2e-ha-replica-election:
	./scripts/e2e-ha-replica-election.sh
