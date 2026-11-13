# Multi-arch server image: docker buildx build --platform linux/amd64,linux/arm64 -f deploy/Dockerfile .
FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# src-tauri is excluded from the server workspace
COPY --from=web /src/web/dist ./web/dist
RUN cargo build --release -p aircraft-monitor-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /data --uid 10001 monitor
COPY --from=build /src/target/release/aircraft-monitor-server /usr/local/bin/aircraft-monitor-server
USER monitor
ENV AM_DATA_DIR=/data AM_BIND=0.0.0.0:8080
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "--", "aircraft-monitor-server"]
