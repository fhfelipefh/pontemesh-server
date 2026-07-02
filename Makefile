.PHONY: check i18n-check api-contract architecture-check

check:
	$(MAKE) architecture-check
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
