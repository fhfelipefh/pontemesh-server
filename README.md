# Ponte Mesh Server

**Ponte Mesh Server** is the server component of **Ponte Mesh**, an open-source framework for hybrid distribution of digital objects with centralized control, fragment-based delivery, and automatic fallback to the origin server.

The project combines the operational simplicity of a client-server architecture with the efficiency of auxiliary distribution sources, such as **Replica/Edge** nodes and authorized peers, while keeping the **Origin** as the system's central authority.

In this architecture, the **Origin** remains responsible for ingestion, cataloging, authentication, authorization, manifest generation, revocation, expiration policies, metrics, and fallback.

The data plane can use auxiliary sources to distribute content fragments whenever that strategy is secure, authorized, and technically advantageous.

`pontemesh-server` is a single application. The same codebase, binary, Docker image, and administration panel operate as an **Origin** or **Replica/Edge** according to the instance's persisted configuration.

## Why this project exists

Traditional client-server architectures concentrate all traffic on the origin server. This approach simplifies control, security, and operational predictability, but it can increase bandwidth costs, place additional load on central infrastructure, and make scaling more difficult when many clients access content simultaneously.

Ponte Mesh provides an intermediate model: it preserves the **Origin** as the central control point while allowing data transfer to be partially decentralized through fragments.

This reduces exclusive dependence on the origin server when trusted and authorized auxiliary sources are available. The **Origin** ensures that retrieval can continue whenever auxiliary delivery is unavailable or unsuitable.

## Main components

### Origin

The central server of the architecture.

It controls publishing, ingestion, primary storage, the object catalog, authentication, authorization, manifest generation, revocation, expiration policies, metrics, and fallback.

The Origin is the system authority. Content retrieval must not occur without prior authorization issued by it.

### Replica/Edge

An auxiliary node with greater operational stability.

Its role is to replicate authorized content and assist with fragment delivery, reducing exclusive dependence on the Origin and ordinary peers.

A Replica/Edge operates under authorization from the Origin, using authenticated, auditable, and revocable communication.

### SDK

The integration layer used by client applications.

The SDK abstracts the complexity of hybrid distribution. It consults the Origin, interprets manifests, selects sources, retrieves fragments, validates integrity, tracks progress, and performs automatic fallback when necessary.

### Client

An application that consumes digital objects.

The Client uses the SDK to access content without handling the complexity of the hybrid architecture directly. When allowed by Origin policies, it can also temporarily share fragments it has already retrieved.

## Project principles

- Every content retrieval must begin with authorization from the Origin.
- The Origin is the central authority for publishing, availability, authentication, authorization, and revocation.
- P2P is an acceleration mechanism subordinate to centralized control.
- Replica/Edge nodes improve availability within the scope authorized by the Origin.
- Every fragment received from any source must pass integrity validation before it is accepted.
- Accepted fragments must match the authorized manifest.
- Revocation and expiration must prevent new access authorizations.
- Fallback to the Origin must preserve fragments that have already been validated instead of restarting the entire retrieval.
- The architecture must remain predictable when peers are unavailable, unstable, or behind NAT and firewalls.
- Whenever possible, the API should be familiar to integrations inspired by the S3 model.

## Architectural goals

Ponte Mesh Server is designed so that:

1. Control remains centralized in the Origin.
2. Data transfer can occur through a hybrid delivery model.
3. Objects are distributed as verifiable fragments.
4. The SDK hides the complexity of hybrid retrieval.
5. The system remains functional without P2P.
6. Authorization respects revocation and expiration.
7. Fallback to the Origin is a fundamental part of the expected behavior.
8. Integration with existing applications remains simple and close to familiar object-storage models.

## Documentation

The public documentation and project website are available at:

<https://fhfelipefh.github.io/pontemesh-docs/>

The documentation repository is available at:

<https://github.com/fhfelipefh/pontemesh-docs>

## Build

To build the project locally, first build the web panel:

```bash
cd web
npm install
npm run build
cd ..
```

Then compile the Rust server:

```bash
cargo build
```

The server requires PostgreSQL. The following variable is mandatory:

```text
PONTEMESH_DATABASE_URL=postgres://pontemesh:pontemesh@postgres:5432/pontemesh
```

In Docker, `postgres` is the service name on the dedicated network. When running directly outside Docker, replace the URL host with an address through which the process can reach PostgreSQL.

The application uses PostgreSQL as its database and fails during startup if the connection is unavailable.

To produce an optimized binary:

```bash
cargo build --release
```

The executable is generated at:

```text
target/release/pontemesh-server
```

## Run

To run in a local environment:

```bash
cargo run
```

Or, after a release build:

```bash
./target/release/pontemesh-server
```

To prepare a local instance for administration by MCP-compatible AI clients, use:

```bash
./target/release/pontemesh setup-agent
```

`setup-agent` completes Origin setup when necessary, enables MCP with mandatory authentication, creates a new MCP token, and writes the connection configuration to `$PONTEMESH_HOME/secrets/setup-agent-mcp.json` with restricted permissions. By default, the generated MCP endpoint is limited to localhost.

The server uses these defaults:

```text
PONTEMESH_HOME=/var/pontemesh_home
PONTEMESH_DATABASE_URL=<required>
PONTEMESH_STORAGE_PATH=/var/pontemesh_home/data/storage
PONTEMESH_HTTP_HOST=0.0.0.0
PONTEMESH_WEB_PORT=8080
PONTEMESH_S3_PORT=9000
```

`PONTEMESH_HOME` is the instance's persistent directory. In containers, mount a volume at `/var/pontemesh_home`; the default storage directory will be created at `/var/pontemesh_home/data/storage`. To use a specific host directory, mount it at `/var/pontemesh_home`, or set `PONTEMESH_STORAGE_PATH` to a prepared internal path.

You can also run the server with Docker:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml up -d --build
```

The `docker-compose.yml` file starts PostgreSQL and passes `PONTEMESH_DATABASE_URL=postgres://pontemesh:pontemesh@postgres:5432/pontemesh` to the server.

There is a single Docker image: `pontemesh-server`. Environments with an Origin and one or more replicas must run multiple instances of the same image, each with its own `PONTEMESH_HOME`, persisted database and configuration, and network parameters.

Docker Compose starts the `ponte-mesh` project with `server` and `postgres` services grouped as one application in Docker Desktop. PostgreSQL remains on the internal Compose network. With PostgreSQL 18, the `pontemesh_postgres` volume is mounted at `/var/lib/postgresql`.

The local Compose configuration uses `docker/Dockerfile.local`, which packages the binary and web build already produced by the script. The multi-stage image in `docker/Dockerfile` builds both the frontend and backend inside the image build.

Access the services at:

```text
Web panel: http://localhost:8080
S3-compatible endpoint: http://localhost:9000
```

## One-command build and startup

The build and startup steps can be run through a single script that prepares the environment and opens the web panel:

```bash
./scripts/start-panel.sh
```

This command:

- installs dependencies and builds the frontend;
- builds the Rust backend in release mode;
- invokes Docker Compose using the `ponte-mesh` project;
- builds the Docker image through Compose;
- starts `server` and `postgres` as services in the same project;
- waits for PostgreSQL to become healthy;
- waits for the server to respond;
- opens `http://localhost:8080` in a new browser tab.

The command uses these defaults:

```text
Docker image: pontemesh-server:local
Compose project: ponte-mesh
Services: server, postgres
Web panel: http://localhost:8080
S3-compatible endpoint: http://localhost:9000
```

You can override these values through environment variables:

```bash
PONTEMESH_WEB_HOST_PORT=8081 \
./scripts/start-panel.sh
```

In this example, the browser opens:

```text
http://localhost:8081
```

To reset a development environment without affecting other projects, use:

```bash
./scripts/start-panel.sh --reset-dev
```

This option resets the Compose project with:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml down --volumes --remove-orphans
```

Back up or migrate production data before removing volumes.

## S3-compatible endpoint

The web administration panel and S3-compatible API use separate ports:

```text
Web/admin panel: http://localhost:8080
S3-compatible API: http://localhost:9000
```

S3 clients use path-style addressing with:

```text
endpoint_url = http://localhost:9000
```

S3-compatible routes are served from the root of the S3 endpoint. S3 clients must use `http://localhost:9000` as the endpoint and paths in the `/{bucket}/{key}` format.

Ponte Mesh SDKs can use temporary access packages issued by the Origin to retrieve objects through dedicated `/pontemesh/access-packages/...` routes, while the S3-compatible endpoint remains separate for S3 clients.

During initial setup, the panel generates an initial S3 access key for the administrator and displays its secret once. Additional keys can later be created or revoked under `Settings > S3 Access Keys`.

Bootstrap variables are optional and are intended only for importing an externally generated key in advanced scenarios:

```bash
export PONTEMESH_S3_BOOTSTRAP_ACCESS_KEY_ID=PMKEXTERNALACCESSKEY
export PONTEMESH_S3_BOOTSTRAP_SECRET_ACCESS_KEY='<secret-generated-outside-pontemesh>'
./scripts/start-panel.sh
```

AWS CLI examples:

```bash
AWS_ACCESS_KEY_ID='<access-key-generated-in-the-panel>' \
AWS_SECRET_ACCESS_KEY='<secret-displayed-once>' \
aws --endpoint-url http://localhost:9000 s3api list-buckets
```

```bash
AWS_ACCESS_KEY_ID='<access-key-generated-in-the-panel>' \
AWS_SECRET_ACCESS_KEY='<secret-displayed-once>' \
aws --endpoint-url http://localhost:9000 s3api create-bucket --bucket test-bucket
```

```bash
AWS_ACCESS_KEY_ID='<access-key-generated-in-the-panel>' \
AWS_SECRET_ACCESS_KEY='<secret-displayed-once>' \
aws --endpoint-url http://localhost:9000 s3api put-object --bucket test-bucket --key hello.txt --body ./hello.txt
```

## Initial setup

On its first run, Ponte Mesh Server creates an initial administrator token.

The token is stored at:

```text
/var/pontemesh_home/secrets/initialAdminToken
```

It also appears in the server logs.

With Docker Compose, view the logs with:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml logs server
```

Or read the token directly:

```bash
docker compose -p ponte-mesh -f docker/docker-compose.yml exec server cat /var/pontemesh_home/secrets/initialAdminToken
```

Then open:

```text
http://localhost:8080
```

Paste the initial token into the web panel and complete the instance setup.

## Support the project

If Ponte Mesh is useful to you, consider supporting its continued development:

[Sponsor on GitHub](https://github.com/sponsors/fhfelipefh)

## Project links

- [Ponte Mesh documentation](https://fhfelipefh.github.io/pontemesh-docs/)
- [Ponte Mesh Server](https://github.com/fhfelipefh/pontemesh-server)
- [Ponte Mesh SDK](https://github.com/fhfelipefh/pontemesh-sdk)
- [Game Launcher Example](https://github.com/fhfelipefh/pontemesh-game-launcher-example)
