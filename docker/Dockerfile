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
# Chainguard glibc-dynamic ships a continuously-rebuilt :latest tag that is the
# documented consumption pattern (cgr.dev does not publish stable semver tags
# for this image). Pin advisories should target the cgr.dev/chainguard SBOM,
# not the image tag.
# hadolint ignore=DL3007
FROM cgr.dev/chainguard/glibc-dynamic:latest

COPY --from=builder /usr/local/bin/liter-llm /usr/bin/liter-llm

ENV RUST_LOG=info

EXPOSE 4000

LABEL org.opencontainers.image.source="https://github.com/kreuzberg-dev/liter-llm"
LABEL org.opencontainers.image.description="LiterLLM — OpenAI-compatible LLM proxy and MCP server with 142+ providers"
LABEL org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["liter-llm"]
CMD ["api", "--host", "0.0.0.0", "--port", "4000"]
