# docker/hyperlane/Dockerfile
# syntax=docker/dockerfile:1.4
# Multi-stage build for Hyperlane Rust agents (left-curve fork, branch dango)
#
# Stage 1: Build Rust binaries from the Hyperlane fork
# Stage 2: Create a minimal runtime image with only the required binaries

#########################
#     BASE + TOOLS      #
#########################
# Use Rust 1.89.0 to match Hyperlane repo toolchain and avoid downloading it during build
FROM rust:1.89-bookworm AS base

# Install necessary build tools
RUN apt-get update && \
    apt-get install -y --no-install-recommends clang git curl ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install sccache binary (faster than compiling with cargo install)
ARG SCCACHE_VERSION=0.12.0
RUN set -eux; \
    ARCH=$(uname -m); \
    case "$ARCH" in \
      x86_64) SCCACHE_ARCH="x86_64" ;; \
      aarch64|arm64) SCCACHE_ARCH="aarch64" ;; \
      *) echo "Unsupported arch: $ARCH"; exit 1 ;; \
    esac; \
    curl -fL -o /tmp/sccache.tar.gz "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-${SCCACHE_ARCH}-unknown-linux-musl.tar.gz"; \
    tar -xzf /tmp/sccache.tar.gz -C /tmp; \
    mv /tmp/sccache-v${SCCACHE_VERSION}-${SCCACHE_ARCH}-unknown-linux-musl/sccache /usr/local/bin/sccache; \
    chmod +x /usr/local/bin/sccache; \
    rm -rf /tmp/sccache*

# Configure sccache for faster builds
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache

#########################
#        BUILDER        #
#########################
FROM base AS builder

# Arguments for selecting the Hyperlane fork and branch/SHA
# These can be overridden from the CI workflow
ARG HYPERLANE_REPO=https://github.com/left-curve/hyperlane-monorepo.git
ARG HYPERLANE_REF=dango
ARG HYPERLANE_SHA=

# Clone the Hyperlane fork repository
WORKDIR /usr/src
RUN if [ -n "${HYPERLANE_SHA}" ]; then \
      git clone "${HYPERLANE_REPO}" hyperlane && \
      cd hyperlane && \
      git checkout "${HYPERLANE_SHA}"; \
    else \
      git clone --depth 1 --branch "${HYPERLANE_REF}" "${HYPERLANE_REPO}" hyperlane; \
    fi

# Navigate into the workspace (same structure as the original Hyperlane repo)
WORKDIR /usr/src/hyperlane/rust/main

# Build required agent binaries (validator, relayer, scraper)
# Uses the same caching mounts as upstream to speed up CI builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    RUSTFLAGS="--cfg tokio_unstable" \
    cargo build --release --bin validator --bin relayer --bin scraper && \
    mkdir -p /release && \
    cp target/release/validator /release && \
    cp target/release/relayer /release && \
    cp target/release/scraper /release

#########################
#        RUNTIME        #
#########################
FROM ubuntu:24.04

# Commit identifier (injected from the CI workflow)
ARG GIT_COMMIT=unknown

# Useful metadata for OCI registries
LABEL org.opencontainers.image.description="Hyperlane Rust agents (left-curve fork, branch dango)"
LABEL org.opencontainers.image.revision=$GIT_COMMIT
LABEL org.opencontainers.image.source="https://github.com/left-curve/hyperlane-monorepo"

ENV GIT_COMMIT=$GIT_COMMIT

WORKDIR /app

# Copy configuration files from the cloned Hyperlane repo
COPY --from=builder /usr/src/hyperlane/rust/main/config /app/config
COPY --from=builder /usr/src/hyperlane/rust/main/app-contexts /app/app-contexts

# Copy compiled binaries from the builder stage
COPY --from=builder /release/* .

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends openssl ca-certificates tini libcurl4 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    chmod 755 /app && \
    mkdir -p /usr/share/hyperlane && chmod 755 /usr/share/hyperlane && \
    mkdir -p /data && chown -R 1000 /data

# Run as non-root user for improved security
USER 1000

# Use tini as the init process (same as Hyperlane upstream)
ENTRYPOINT ["tini", "--"]

# Default command (can be overridden with docker run ...)
CMD ["./validator"]
