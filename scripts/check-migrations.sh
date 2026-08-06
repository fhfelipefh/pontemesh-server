#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATIONS_DIR="$ROOT_DIR/migrations"

fail() {
  printf 'migration guard failed: %s\n' "$1" >&2
  exit 1
}

[ -d "$MIGRATIONS_DIR" ] || fail "migrations directory not found"

invalid_names="$(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*.sql' \
  | sed 's#.*/##' \
  | grep -Ev '^[0-9]+_[A-Za-z0-9_ -]+\.sql$' || true)"

if [ -n "$invalid_names" ]; then
  printf '%s\n' "$invalid_names" >&2
  fail "migration filenames must start with a numeric version prefix"
fi

duplicates="$(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*.sql' \
  | sed 's#.*/##' \
  | sed 's/_.*//' \
  | sort \
  | uniq -d)"

if [ -n "$duplicates" ]; then
  printf 'Duplicate migration versions:\n' >&2
  for version in $duplicates; do
    printf '%s\n' "$version" >&2
    find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name "${version}_*.sql" \
      | sed 's#.*/#  #' \
      | sort >&2
  done
  fail "SQLx migration versions must be unique"
fi

empty_files="$(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*.sql' -empty || true)"

if [ -n "$empty_files" ]; then
  printf '%s\n' "$empty_files" >&2
  fail "empty migration files are not allowed"
fi

printf 'migration guard ok\n'
