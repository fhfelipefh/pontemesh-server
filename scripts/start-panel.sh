#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

COMPOSE_PROJECT_NAME="${PONTEMESH_COMPOSE_PROJECT_NAME:-ponte-mesh}"
COMPOSE_FILE="${PONTEMESH_COMPOSE_FILE:-docker/docker-compose.yml}"
WEB_HOST_PORT="${PONTEMESH_WEB_HOST_PORT:-8080}"
S3_HOST_PORT="${PONTEMESH_S3_HOST_PORT:-9000}"
WEB_URL="http://localhost:${WEB_HOST_PORT}"
S3_URL="http://localhost:${S3_HOST_PORT}"

log() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

usage() {
  cat <<EOF
Usage: $0 [--reset-dev]

Options:
  --reset-dev  Remove only the Ponte Mesh Compose project resources with:
               docker compose -p ${COMPOSE_PROJECT_NAME} -f ${COMPOSE_FILE} down --volumes --remove-orphans
EOF
}

compose() {
  docker compose -p "$COMPOSE_PROJECT_NAME" -f "$COMPOSE_FILE" "$@"
}

open_browser() {
  local url="$1"

  if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 &
    return 0
  fi

  if command -v gio >/dev/null 2>&1; then
    gio open "$url" >/dev/null 2>&1 &
    return 0
  fi

  if command -v sensible-browser >/dev/null 2>&1; then
    sensible-browser "$url" >/dev/null 2>&1 &
    return 0
  fi

  if command -v google-chrome >/dev/null 2>&1; then
    google-chrome --new-tab "$url" >/dev/null 2>&1 &
    return 0
  fi

  if command -v open >/dev/null 2>&1; then
    open "$url" >/dev/null 2>&1 &
    return 0
  fi

  printf 'Could not open a browser automatically. Open this URL manually: %s\n' "$url" >&2
}

wait_for_http() {
  local url="$1"
  local service="$2"
  local attempts=60

  for _ in $(seq 1 "$attempts"); do
    if curl --silent --fail --output /dev/null "$url"; then
      return 0
    fi
    sleep 1
  done

  printf 'Server did not respond at %s within %s seconds.\n' "$url" "$attempts" >&2
  printf 'Compose service logs (%s):\n' "$service" >&2
  compose logs --no-color "$service" >&2 || true
  exit 1
}

RESET_DEV=0
for arg in "$@"; do
  case "$arg" in
    --reset-dev)
      RESET_DEV=1
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

require_command npm
require_command cargo
require_command docker
require_command curl

cd "$ROOT_DIR"

log "Checking PostgreSQL migrations"
./scripts/check-migrations.sh

if [ "$RESET_DEV" -eq 1 ]; then
  log "Resetting Ponte Mesh Compose project"
  compose down --volumes --remove-orphans
fi

log "Installing frontend dependencies"
npm install --prefix web

log "Building frontend"
npm run build --prefix web

log "Building backend"
cargo build --release

log "Starting Ponte Mesh Compose project"
compose up -d --build

log "Waiting for Ponte Mesh web panel at ${WEB_URL}"
wait_for_http "$WEB_URL" server

log "Opening ${WEB_URL}"
open_browser "$WEB_URL"

printf '\nPonte Mesh is running as Docker Compose project: %s\n' "$COMPOSE_PROJECT_NAME"
printf 'Web panel: %s\n' "$WEB_URL"
printf 'S3-compatible endpoint: %s\n' "$S3_URL"
printf 'Services:\n'
printf '  server\n'
printf '  postgres\n'
printf 'Initial setup token, when needed:\n'
printf '  docker compose -p %s -f %s logs server\n' "$COMPOSE_PROJECT_NAME" "$COMPOSE_FILE"
printf '  docker compose -p %s -f %s exec server cat /var/pontemesh_home/secrets/initialAdminToken\n' "$COMPOSE_PROJECT_NAME" "$COMPOSE_FILE"
