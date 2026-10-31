# syntax=docker/dockerfile:1
FROM rust:1.99.1-trixie AS builder

ARG TARGETARCH

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

RUN --mount=type=cache,id=cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git-${TARGETARCH},target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target-${TARGETARCH},target=/build/target \
    cargo build --release --package liter-llm-cli --features otel && \
    cp target/release/liter-llm /usr/local/bin/liter-llm

# cgr.dev/chainguard/glibc-dynamic:latest
FROM cgr.dev/chainguard/glibc-dynamic@sha256:709673274b417a48f56374da8a7633ef7b2f5b27e34612ce98a335737318cc94

COPY --from=builder /usr/local/bin/liter-llm /usr/bin/liter-llm

ENV RUST_LOG=info

EXPOSE 4000

LABEL org.opencontainers.image.source="https://github.com/xberg-io/liter-llm"
LABEL org.opencontainers.image.description="LiterLLM — OpenAI-compatible LLM proxy and MCP server with 174 providers"
LABEL org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["liter-llm"]
CMD ["api", "--host", "0.0.0.0", "--port", "4000"]
