#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_PROJECT_NAME="pontemesh-ha-e2e"
COMPOSE_FILE="docker/docker-compose.e2e-ha.yml"
ORIGIN_URL="http://127.0.0.1:18180"
ADMIN_USER="admin"
ADMIN_PASS="correct-password"
BUCKET="e2e-bucket"
KEY="folder/ha-object.bin"
PAYLOAD="/tmp/pontemesh-ha-payload.bin"
ADMIN_COOKIE="/tmp/pontemesh-ha-admin.cookie"
SETUP_COOKIE="/tmp/pontemesh-ha-setup.cookie"
KEEP_RUNNING=0

usage() {
  cat <<EOF
Usage: $0 [--keep-running] [--reset]

Runs a full high-availability E2E scenario with 1 Origin and 5 Replica/Edge
instances. The test creates a bucket, uploads an object, registers 5 replicas,
waits until every replica synchronizes the object, validates serving from all
replicas, stops the Origin, then validates degraded data-plane leadership.

Options:
  --keep-running  Leave the Compose project running after the test.
  --reset         Only remove this E2E Compose project and exit.
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

cleanup() {
  if [ "$KEEP_RUNNING" -eq 0 ]; then
    compose down --volumes --remove-orphans >/dev/null 2>&1 || true
  fi
}

wait_http() {
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
  compose logs --no-color --tail=120 "$service" >&2 || true
  exit 1
}

json_get() {
  python3 -c 'import json,sys; data=json.load(sys.stdin); cur=data
for part in sys.argv[1].split("."):
    cur=cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1"
}

setup_instance() {
  local service="$1"
  local url="$2"
  local body="$3"
  local token
  token="$(compose exec -T "$service" cat /var/pontemesh_home/secrets/initialAdminToken)"
  rm -f "$SETUP_COOKIE"
  curl --silent --show-error --fail -c "$SETUP_COOKIE" \
    -H 'content-type: application/json' \
    -d "{\"token\":\"$token\"}" \
    "$url/api/setup/unlock" >/dev/null
  curl --silent --show-error --fail -b "$SETUP_COOKIE" \
    -H 'content-type: application/json' \
    -d "$body" \
    "$url/api/setup/complete" >/dev/null
}

admin_post() {
  local path="$1"
  local body="$2"
  curl --silent --show-error --fail -b "$ADMIN_COOKIE" \
    -H 'content-type: application/json' \
    -d "$body" \
    "$ORIGIN_URL$path"
}

admin_put() {
  local path="$1"
  local body="$2"
  curl --silent --show-error --fail -X PUT -b "$ADMIN_COOKIE" \
    -H 'content-type: application/json' \
    -d "$body" \
    "$ORIGIN_URL$path"
}

app_post() {
  local token="$1"
  local path="$2"
  local body="$3"
  curl --silent --show-error --fail \
    -H "authorization: Bearer $token" \
    -H 'content-type: application/json' \
    -d "$body" \
    "$ORIGIN_URL$path"
}

RESET_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --keep-running)
      KEEP_RUNNING=1
      ;;
    --reset)
      RESET_ONLY=1
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
require_command python3
require_command sha256sum

cd "$ROOT_DIR"

if [ "$RESET_ONLY" -eq 1 ]; then
  log "Resetting HA E2E Compose project"
  compose down --volumes --remove-orphans
  exit 0
fi

trap cleanup EXIT

log "Checking PostgreSQL migrations"
./scripts/check-migrations.sh

log "Building frontend"
npm run build --prefix web

log "Building release backend used by Dockerfile.local"
cargo build --release --bin pontemesh-server

log "Starting 1 Origin + 5 Replica/Edge Compose project"
compose down --volumes --remove-orphans >/dev/null 2>&1 || true
compose up -d --build

log "Waiting for HTTP listeners"
wait_http "$ORIGIN_URL" origin-server
for i in 1 2 3 4 5; do
  wait_http "http://127.0.0.1:1818$i" "replica-$i"
done

log "Completing Origin setup"
setup_instance origin-server "$ORIGIN_URL" "{\"instanceName\":\"Origin E2E\",\"role\":\"origin\",\"adminUsername\":\"$ADMIN_USER\",\"adminPassword\":\"$ADMIN_PASS\",\"httpPort\":8080}"

log "Logging in as admin"
rm -f "$ADMIN_COOKIE"
curl --silent --show-error --fail -c "$ADMIN_COOKIE" \
  -H 'content-type: application/json' \
  -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\"}" \
  "$ORIGIN_URL/api/auth/login" >/dev/null

log "Creating bucket, replica policy and test object"
admin_post "/api/admin/buckets" "{\"name\":\"$BUCKET\"}" >/dev/null
admin_put "/api/admin/buckets/$BUCKET/policy" '{"accessPackageTtlSeconds":300,"fragmentSizeBytes":1024,"allowReplicaEdge":true,"allowPeerSharing":false,"sourceSelectionStrategy":"ORIGIN_REPLICA_EDGE","fragmentPriorityStrategy":"MANIFEST_ORDER","failureThreshold":2,"fallbackMode":"ORIGIN_RANGE"}' >/dev/null
python3 -c 'from pathlib import Path; Path("/tmp/pontemesh-ha-payload.bin").write_bytes((b"pontemesh-five-replica-e2e-" * 180)[:4096])'
curl --silent --show-error --fail -b "$ADMIN_COOKIE" \
  -F "key=$KEY" \
  -F "file=@$PAYLOAD;type=application/octet-stream" \
  "$ORIGIN_URL/api/admin/buckets/$BUCKET/objects" >/dev/null
payload_sha="$(sha256sum "$PAYLOAD" | awk '{print $1}')"
payload_size="$(wc -c < "$PAYLOAD")"

log "Creating 5 replica credentials and completing replica setup"
declare -a replica_ids
declare -a replica_tokens
for i in 1 2 3 4 5; do
  created="$(admin_post "/api/admin/replicas" "{\"name\":\"edge-$i\",\"allowedBuckets\":[\"$BUCKET\"]}")"
  replica_ids[$i]="$(printf '%s' "$created" | json_get 'replica.id')"
  replica_tokens[$i]="$(printf '%s' "$created" | json_get 'token')"
  setup_instance "replica-$i" "http://127.0.0.1:1818$i" "{\"instanceName\":\"Replica $i\",\"role\":\"replica-edge\",\"adminUsername\":\"admin\",\"adminPassword\":\"$ADMIN_PASS\",\"httpPort\":8080,\"originBaseUrl\":\"http://origin-server:8080\",\"replicaId\":\"${replica_ids[$i]}\",\"replicaToken\":\"${replica_tokens[$i]}\",\"replicaPublicEndpoint\":\"http://127.0.0.1:1818$i\",\"syncIntervalSeconds\":5,\"healthIntervalSeconds\":5}"
done

log "Restarting replicas so replica_runtime starts"
compose restart replica-1 replica-2 replica-3 replica-4 replica-5 >/dev/null
for i in 1 2 3 4 5; do
  wait_http "http://127.0.0.1:1818$i" "replica-$i"
done

log "Waiting for all replicas to sync local state"
deadline=$((SECONDS + 90))
while true; do
  synced=0
  for i in 1 2 3 4 5; do
    if compose exec -T "replica-$i" sh -lc "test -f /var/pontemesh_home/data/storage/replica/state.json && grep -q '$KEY' /var/pontemesh_home/data/storage/replica/state.json" >/dev/null 2>&1; then
      synced=$((synced + 1))
    fi
  done
  if [ "$synced" -eq 5 ]; then
    break
  fi
  if [ "$SECONDS" -gt "$deadline" ]; then
    printf 'Only %s/5 replicas synced before timeout.\n' "$synced" >&2
    compose logs --no-color --tail=120 replica-1 replica-2 replica-3 replica-4 replica-5 >&2 || true
    exit 1
  fi
  sleep 2
done

log "Creating SDK credential and access package"
app_created="$(admin_post "/api/admin/application-credentials" '{"name":"sdk-e2e","scopes":["origin:objects:read","pontemesh:manifest:read","pontemesh:availability:read","pontemesh:sources:read","pontemesh:policies:read","pontemesh:access-package:create"]}')"
app_token="$(printf '%s' "$app_created" | json_get 'token')"
package="$(app_post "$app_token" "/pontemesh/access-packages" "{\"bucket\":\"$BUCKET\",\"key\":\"$KEY\",\"ttlSeconds\":300}")"
package_id="$(printf '%s' "$package" | json_get 'id')"
package_token="$(printf '%s' "$package" | json_get 'packageToken')"
source_count="$(printf '%s' "$package" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["authorizedSources"]))')"

log "Validating normal object serve from all replicas"
normal_ok=0
for i in 1 2 3 4 5; do
  out="/tmp/pontemesh-replica-$i-normal.bin"
  code="$(curl --silent --show-error --output "$out" --write-out '%{http_code}' -H "authorization: Bearer $package_token" "http://127.0.0.1:1818$i/pontemesh/replica/access-packages/$package_id/objects/$BUCKET/$KEY")"
  got_sha="$(sha256sum "$out" | awk '{print $1}')"
  if [ "$code" = "200" ] && [ "$got_sha" = "$payload_sha" ]; then
    normal_ok=$((normal_ok + 1))
  else
    printf 'replica-%s normal serve failed: status=%s sha=%s expected=%s\n' "$i" "$code" "$got_sha" "$payload_sha" >&2
    exit 1
  fi
done

availability="$(curl --silent --show-error --fail -H "authorization: Bearer $app_token" "$ORIGIN_URL/pontemesh/objects/$BUCKET/availability/$KEY")"
replica_sources="$(printf '%s' "$availability" | json_get 'replicaSources')"

log "Stopping Origin to test degraded leader behavior"
compose stop origin-server >/dev/null
sleep 2
degraded_ok=0
degraded_unavailable=0
leader_port=""
for i in 1 2 3 4 5; do
  out="/tmp/pontemesh-replica-$i-degraded.bin"
  headers="/tmp/pontemesh-replica-$i-degraded.headers"
  code="$(curl --silent --show-error --dump-header "$headers" --output "$out" --write-out '%{http_code}' -H "authorization: Bearer $package_token" "http://127.0.0.1:1818$i/pontemesh/replica/access-packages/$package_id/objects/$BUCKET/$KEY" || true)"
  if [ "$code" = "200" ]; then
    got_sha="$(sha256sum "$out" | awk '{print $1}')"
    grep -qi '^x-pontemesh-degraded-leader: true' "$headers"
    [ "$got_sha" = "$payload_sha" ]
    degraded_ok=$((degraded_ok + 1))
    leader_port="1818$i"
  elif [ "$code" = "503" ]; then
    degraded_unavailable=$((degraded_unavailable + 1))
  else
    printf 'unexpected degraded response from replica-%s: %s\n' "$i" "$code" >&2
    cat "$headers" >&2 || true
    exit 1
  fi
done

printf '\nRESULT payload_size=%s payload_sha=%s synced_replicas=%s normal_serves=%s package_sources=%s origin_availability_replica_sources=%s degraded_leaders=%s degraded_standbys=%s leader_port=%s\n' \
  "$payload_size" "$payload_sha" "$synced" "$normal_ok" "$source_count" "$replica_sources" "$degraded_ok" "$degraded_unavailable" "$leader_port"

if [ "$synced" -ne 5 ] || [ "$normal_ok" -ne 5 ] || [ "$replica_sources" -ne 5 ] || [ "$degraded_ok" -ne 1 ] || [ "$degraded_unavailable" -ne 4 ]; then
  printf 'HA replica election E2E assertions failed\n' >&2
  exit 1
fi
