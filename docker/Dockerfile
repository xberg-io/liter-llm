# syntax=docker/dockerfile:1
# ── Builder ──────────────────────────────────────────────────────────
FROM rust:1.91-bookworm AS builder

ARG TARGETARCH

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

# Per-platform cache IDs prevent race conditions in multi-arch builds.
RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git-${TARGETARCH},target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target-${TARGETARCH},target=/build/target \
    cargo build --release --package liter-llm-cli && \
    cp target/release/liter-llm /usr/local/bin/liter-llm

# ── Runtime (Chainguard) ─────────────────────────────────────────────
# Pinned multi-platform index for cgr.dev/chainguard/glibc-dynamic.
FROM cgr.dev/chainguard/glibc-dynamic@sha256:7ff79e2caef2b8a137ddaf9940fb790e91148482092363760d6661e4591fd54c

COPY --from=builder /usr/local/bin/liter-llm /usr/bin/liter-llm

ENV RUST_LOG=info

EXPOSE 4000

LABEL org.opencontainers.image.source="https://github.com/xberg-io/liter-llm"
LABEL org.opencontainers.image.description="LiterLLM — OpenAI-compatible LLM proxy and MCP server with 142+ providers"
LABEL org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["liter-llm"]
CMD ["api", "--host", "0.0.0.0", "--port", "4000"]
