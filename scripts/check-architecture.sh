#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'architecture guard failed: %s\n' "$1" >&2
  exit 1
}

if [ -d src/bin ]; then
  fail "src/bin would create extra server binaries"
fi

if rg -n -g '!scripts/check-architecture.sh' '^\[features\]|cfg\s*\(\s*feature\s*=\s*"(origin|replica|replication|standalone)"|--features\s+(origin|replica|replication|standalone)' Cargo.toml src docker scripts >/tmp/pontemesh-architecture-rg.txt; then
  cat /tmp/pontemesh-architecture-rg.txt >&2
  fail "runtime roles must not be selected by cargo features"
fi

if rg -n -g '!scripts/check-architecture.sh' 'standalone|Standalone|stand-alone' src config docker scripts Cargo.toml >/tmp/pontemesh-standalone-rg.txt; then
  cat /tmp/pontemesh-standalone-rg.txt >&2
  fail "standalone mode must not be reintroduced"
fi

entrypoint_count="$(rg -n 'ENTRYPOINT \["/usr/local/bin/pontemesh-server"\]' docker/Dockerfile | wc -l | tr -d ' ')"
[ "$entrypoint_count" = "1" ] || fail "Dockerfile must keep one pontemesh-server entrypoint"

printf 'architecture guard ok\n'
