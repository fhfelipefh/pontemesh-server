.PHONY: check

check:
	cd web && npm run validate
	cargo check
