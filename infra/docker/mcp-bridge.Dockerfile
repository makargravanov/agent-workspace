FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml ./
COPY services ./services
COPY tools ./tools

RUN cargo build --release -p agent-workspace-mcp

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/agent-workspace-mcp /usr/local/bin/agent-workspace-mcp

CMD ["agent-workspace-mcp"]
