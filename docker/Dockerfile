FROM node:22-bookworm-slim AS web-build

WORKDIR /app
COPY web/package.json web/package-lock.json* ./web/
WORKDIR /app/web
RUN npm ci
COPY web ./
RUN npm run i18n:check && npm run typecheck && npm run lint && npm run test && npm run build:app

FROM rust:1-bookworm AS rust-build

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
COPY --from=web-build /app/web/dist ./web/dist
RUN cargo build --release

FROM debian:bookworm-slim

ENV PONTEMESH_HOME=/var/pontemesh_home
ENV RUST_LOG=pontemesh_server=info,tower_http=info

RUN useradd --system --create-home --home-dir /nonexistent --shell /usr/sbin/nologin pontemesh \
    && mkdir -p /var/pontemesh_home \
    && chown -R pontemesh:pontemesh /var/pontemesh_home

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-build /app/target/release/pontemesh-server /usr/local/bin/pontemesh-server
COPY --from=rust-build /app/migrations ./migrations

USER pontemesh
EXPOSE 8080 9000
VOLUME ["/var/pontemesh_home"]

ENTRYPOINT ["/usr/local/bin/pontemesh-server"]
