# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.82-slim-bookworm AS builder

# System deps for SQLite (native library) and ring (C crypto)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libsqlite3-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --bin proxy --bin dashboard

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false llmproxy

WORKDIR /app
COPY --from=builder /build/target/release/proxy    /app/proxy
COPY --from=builder /build/target/release/dashboard /app/dashboard
COPY migrations/ /app/migrations/

RUN chown -R llmproxy:llmproxy /app
USER llmproxy

# proxy listens on 8080 by default; dashboard on 8081
EXPOSE 8080 8081

# Default: run the proxy.  Override CMD to run dashboard instead.
CMD ["/app/proxy"]
