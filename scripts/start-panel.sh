#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

IMAGE_NAME="${PONTEMESH_IMAGE_NAME:-pontemesh-server:local}"
CONTAINER_NAME="${PONTEMESH_CONTAINER_NAME:-pontemesh-server}"
VOLUME_NAME="${PONTEMESH_VOLUME_NAME:-pontemesh_home}"
HOST_PORT="${PONTEMESH_HOST_PORT:-8080}"
CONTAINER_PORT="8080"
APP_URL="http://localhost:${HOST_PORT}"

log() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
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
  local attempts=60

  for _ in $(seq 1 "$attempts"); do
    if curl --silent --fail --output /dev/null "$url"; then
      return 0
    fi
    sleep 1
  done

  printf 'Server did not respond at %s within %s seconds.\n' "$url" "$attempts" >&2
  printf 'Container logs:\n' >&2
  docker logs "$CONTAINER_NAME" >&2 || true
  exit 1
}

require_command npm
require_command cargo
require_command docker
require_command curl

cd "$ROOT_DIR"

log "Installing frontend dependencies"
npm install --prefix web

log "Building frontend"
npm run build --prefix web

log "Building backend"
cargo build --release

log "Building Docker image"
docker build -f docker/Dockerfile -t "$IMAGE_NAME" .

log "Starting Docker container"
if docker ps -a --format '{{.Names}}' | grep -Fxq "$CONTAINER_NAME"; then
  docker rm -f "$CONTAINER_NAME" >/dev/null
fi

docker run \
  --detach \
  --name "$CONTAINER_NAME" \
  --publish "${HOST_PORT}:${CONTAINER_PORT}" \
  --volume "${VOLUME_NAME}:/var/pontemesh_home" \
  "$IMAGE_NAME" >/dev/null

log "Waiting for Ponte Mesh at ${APP_URL}"
wait_for_http "$APP_URL"

log "Opening ${APP_URL}"
open_browser "$APP_URL"

printf '\nPonte Mesh is running at %s\n' "$APP_URL"
printf 'Container: %s\n' "$CONTAINER_NAME"
printf 'Initial setup token, when needed:\n'
printf '  docker logs %s\n' "$CONTAINER_NAME"
printf '  docker exec %s cat /var/pontemesh_home/secrets/initialAdminToken\n' "$CONTAINER_NAME"
