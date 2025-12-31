FROM rust:1.83-slim-bookworm as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./

RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

RUN cargo build --release && rm -rf src target/release/deps/codazzy*
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    snmp \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/codazzy-gateway /app/codazzy-gateway
COPY .env.example /app/.env.example

RUN useradd -r -s /bin/false gateway && \
    chown -R gateway:gateway /app

USER gateway
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["/app/codazzy-gateway"]

