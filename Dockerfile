# Build the Mini App static assets.
FROM node:24-bookworm-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Build the Rust backend. The dummy main warms dependency compilation before
# the real sources are copied.
FROM rust:1.99-bookworm AS backend
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
RUN mkdir -p backend/src && printf 'fn main() {}\n' > backend/src/main.rs
RUN cargo build --release -p raid-scheduler-backend
COPY backend/src backend/src
COPY migrations migrations
RUN touch backend/src/main.rs && cargo build --release -p raid-scheduler-backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend /app/target/release/raid-scheduler-backend /usr/local/bin/raid-scheduler-backend
COPY --from=frontend /app/frontend/dist /app/frontend/dist

RUN useradd --system --uid 10001 --create-home --home-dir /app app \
    && mkdir -p /app/data \
    && chown -R app:app /app

USER app
ENV APP_BIND_ADDR=0.0.0.0:8080 \
    DATABASE_PATH=/app/data/app.db \
    FRONTEND_DIST=/app/frontend/dist \
    RUST_LOG=info

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

CMD ["raid-scheduler-backend"]
