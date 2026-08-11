#!/usr/bin/env bash
set -euo pipefail

SERVER_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_ROOT="${PONTEMESH_SDK_SOURCE:-$SERVER_ROOT/.ci/pontemesh-sdk}"
RUN_ID="${GITHUB_RUN_ID:-local}-$$"
IMAGE="pontemesh-server-integration:$RUN_ID"
NETWORK="pontemesh-integration-$RUN_ID"
POSTGRES="pontemesh-postgres-$RUN_ID"
ORIGIN="pontemesh-origin-$RUN_ID"
ORIGIN_URL="http://127.0.0.1:18080"
WORK="$(mktemp -d)"
ADMIN_PASSWORD="${PONTEMESH_SDK_INTEGRATION_ADMIN_PASSWORD:-Pm$(openssl rand -hex 16)!Aa1}"

cleanup() {
  docker rm -f "$ORIGIN" "$POSTGRES" >/dev/null 2>&1 || true
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

if [[ ! -f "$SDK_ROOT/Cargo.toml" ]]; then
  git clone --depth 1 https://github.com/fhfelipefh/pontemesh-sdk.git "$SDK_ROOT"
fi

docker build -f "$SERVER_ROOT/docker/Dockerfile" -t "$IMAGE" "$SERVER_ROOT"
docker network create "$NETWORK" >/dev/null
docker run -d --name "$POSTGRES" --network "$NETWORK" --network-alias postgres \
  -e POSTGRES_DB=pontemesh -e POSTGRES_USER=pontemesh -e POSTGRES_PASSWORD=pontemesh \
  postgres:18-bookworm >/dev/null
for _ in $(seq 1 60); do
  docker exec "$POSTGRES" pg_isready -U pontemesh -d pontemesh >/dev/null 2>&1 && break
  sleep 1
done

docker run -d --name "$ORIGIN" --network "$NETWORK" -p 18080:8080 \
  -e PONTEMESH_DATABASE_URL=postgres://pontemesh:pontemesh@postgres:5432/pontemesh \
  -e PONTEMESH_HOME=/var/pontemesh_home "$IMAGE" >/dev/null
for _ in $(seq 1 90); do
  curl --silent --fail "$ORIGIN_URL/api/setup/status" >/dev/null 2>&1 && break
  sleep 1
done
curl --silent --fail "$ORIGIN_URL/api/setup/status" >/dev/null

token="$(docker exec "$ORIGIN" cat /var/pontemesh_home/secrets/initialAdminToken)"
curl --silent --fail -c "$WORK/setup.cookies" -H 'content-type: application/json' \
  -d "{\"token\":\"$token\"}" "$ORIGIN_URL/api/setup/unlock" >/dev/null
curl --silent --fail -b "$WORK/setup.cookies" -H 'content-type: application/json' \
  -d "{\"instanceName\":\"SDK integration\",\"role\":\"origin\",\"adminUsername\":\"admin\",\"adminPassword\":\"$ADMIN_PASSWORD\",\"httpPort\":8080,\"internalStoragePath\":\"/var/pontemesh_home/storage\"}" \
  "$ORIGIN_URL/api/setup/complete" >/dev/null
curl --silent --fail -c "$WORK/admin.cookies" -H 'content-type: application/json' \
  -d "{\"username\":\"admin\",\"password\":\"$ADMIN_PASSWORD\"}" "$ORIGIN_URL/api/auth/login" >/dev/null
curl --silent --fail -b "$WORK/admin.cookies" -H 'content-type: application/json' \
  -d '{"name":"sdk-integration"}' "$ORIGIN_URL/api/admin/buckets" >/dev/null
printf 'Ponte Mesh live Server and SDK integration\n' > "$WORK/object.bin"
curl --silent --fail -b "$WORK/admin.cookies" -F 'key=releases/integration.bin' \
  -F "file=@$WORK/object.bin" "$ORIGIN_URL/api/admin/buckets/sdk-integration/objects" >/dev/null
curl --silent --fail -b "$WORK/admin.cookies" -H 'content-type: application/json' \
  -d '{"name":"sdk-ci","preset":"downloader"}' "$ORIGIN_URL/api/admin/application-credentials" \
  > "$WORK/application.json"

export PONTEMESH_LIVE_ORIGIN_URL="$ORIGIN_URL"
export PONTEMESH_LIVE_APPLICATION_TOKEN
PONTEMESH_LIVE_APPLICATION_TOKEN="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["token"])' "$WORK/application.json")"
export PONTEMESH_LIVE_BUCKET="sdk-integration"
export PONTEMESH_LIVE_KEY="releases/integration.bin"
export PONTEMESH_LIVE_EXPECTED_SHA256
PONTEMESH_LIVE_EXPECTED_SHA256="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$WORK/object.bin")"

cd "$SDK_ROOT"
./scripts/sdk-server-integration-gate.sh
