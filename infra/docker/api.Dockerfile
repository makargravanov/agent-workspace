# syntax=docker/dockerfile:1.7

FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY services/api/Cargo.toml ./services/api/Cargo.toml
COPY tools/mcp-bridge/Cargo.toml ./tools/mcp-bridge/Cargo.toml

RUN mkdir -p services/api/src tools/mcp-bridge/src \
    && printf 'fn main() {}\n' > services/api/src/main.rs \
    && printf 'fn main() {}\n' > tools/mcp-bridge/src/main.rs

RUN --mount=type=cache,id=agent-workspace-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=agent-workspace-cargo-git,sharing=locked,target=/usr/local/cargo/git \
    --mount=type=cache,id=agent-workspace-rust-target,sharing=locked,target=/app/target \
    cargo build --locked --release -p agent-workspace-api

COPY services ./services
COPY tools ./tools

RUN --mount=type=cache,id=agent-workspace-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=agent-workspace-cargo-git,sharing=locked,target=/usr/local/cargo/git \
    --mount=type=cache,id=agent-workspace-rust-target,sharing=locked,target=/app/target \
    cargo build --locked --release -p agent-workspace-api \
    && cp /app/target/release/agent-workspace-api /tmp/agent-workspace-api

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /tmp/agent-workspace-api /usr/local/bin/agent-workspace-api

EXPOSE 8080

CMD ["agent-workspace-api"]
