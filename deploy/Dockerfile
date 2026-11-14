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
RUN cargo build --release -p ramp-radar-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /data --uid 10001 monitor
COPY --from=build /src/target/release/ramp-radar-server /usr/local/bin/ramp-radar-server
USER monitor
ENV RR_DATA_DIR=/data RR_BIND=0.0.0.0:8080
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "--", "ramp-radar-server"]
