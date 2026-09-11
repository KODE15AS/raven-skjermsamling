# Skjermsamling – frontend (Svelte) + backend (Rust) i ett image.
# Session-controlleren i backend styrer workspace-containere via Dockers
# API på verten; kun DENNE containeren får docker-socketen, aldri workspaces.

FROM node:22-slim AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM rust:1.98-slim AS backend
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -fsSL https://download.docker.com/linux/static/stable/x86_64/docker-27.3.1.tgz \
       | tar -xz --strip-components=1 -C /usr/local/bin docker/docker \
    && apt-get purge -y curl && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend /app/target/release/skjermsamling /usr/local/bin/skjermsamling
COPY --from=frontend /app/dist /app/static

ENV STATIC_DIR=/app/static \
    DB_PATH=/data/skjermsamling.db \
    BIND=0.0.0.0:8015

VOLUME /data
EXPOSE 8015
CMD ["skjermsamling"]
