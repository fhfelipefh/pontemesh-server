#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_PROJECT_NAME="ponte-mesh-e2e-origin-replica"
COMPOSE_FILE="docker/docker-compose.e2e-origin-replica.yml"
ORIGIN_URL="http://127.0.0.1:18080"
ORIGIN_S3_URL="http://127.0.0.1:19000"
REPLICA_URL="http://127.0.0.1:18081"
REPLICA_S3_URL="http://127.0.0.1:19001"

usage() {
  cat <<EOF
Usage: $0 [--reset]

Options:
  --reset  Remove only the E2E Origin+Replica Compose resources with:
           docker compose -p ${COMPOSE_PROJECT_NAME} -f ${COMPOSE_FILE} down --volumes --remove-orphans
EOF
}

log() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

compose() {
  docker compose -p "$COMPOSE_PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

wait_for_postgres() {
  local service="$1"
  local attempts=60

  for _ in $(seq 1 "$attempts"); do
    if compose exec -T "$service" pg_isready -U pontemesh -d pontemesh >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  printf 'Postgres service did not become healthy: %s\n' "$service" >&2
  compose logs --no-color "$service" >&2 || true
  exit 1
}

wait_for_http() {
  local url="$1"
  local service="$2"
  local attempts=90

  for _ in $(seq 1 "$attempts"); do
    if curl --silent --fail --output /dev/null "$url/api/setup/status"; then
      return 0
    fi
    sleep 1
  done

  printf 'Service did not respond at %s within %s seconds.\n' "$url" "$attempts" >&2
  compose logs --no-color "$service" >&2 || true
  exit 1
}

RESET=0
for arg in "$@"; do
  case "$arg" in
    --reset)
      RESET=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$arg" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_command cargo
require_command curl
require_command docker
require_command npm

cd "$ROOT_DIR"

if [ "$RESET" -eq 1 ]; then
  log "Resetting E2E Origin+Replica Compose project"
  compose down --volumes --remove-orphans
fi

log "Checking PostgreSQL migrations"
./scripts/check-migrations.sh

log "Installing frontend dependencies"
npm install --prefix web

log "Building frontend"
npm run build --prefix web

log "Building backend"
cargo build --release

log "Starting E2E Origin+Replica Compose project"
compose up -d --build

log "Waiting for Origin PostgreSQL"
wait_for_postgres origin-postgres

log "Waiting for Replica PostgreSQL"
wait_for_postgres replica-postgres

log "Waiting for Origin web at ${ORIGIN_URL}"
wait_for_http "$ORIGIN_URL" origin-server

log "Waiting for Replica web at ${REPLICA_URL}"
wait_for_http "$REPLICA_URL" replica-server

printf '\nPonte Mesh Origin+Replica E2E is running as Docker Compose project: %s\n' "$COMPOSE_PROJECT_NAME"
printf 'Origin web:   %s\n' "$ORIGIN_URL"
printf 'Origin S3:    %s\n' "$ORIGIN_S3_URL"
printf 'Replica web:  %s\n' "$REPLICA_URL"
printf 'Replica S3:   %s\n' "$REPLICA_S3_URL"
printf 'Initial setup tokens:\n'
printf '  docker compose -p %s -f %s exec -T origin-server cat /var/pontemesh_home/secrets/initialAdminToken\n' "$COMPOSE_PROJECT_NAME" "$COMPOSE_FILE"
printf '  docker compose -p %s -f %s exec -T replica-server cat /var/pontemesh_home/secrets/initialAdminToken\n' "$COMPOSE_PROJECT_NAME" "$COMPOSE_FILE"
