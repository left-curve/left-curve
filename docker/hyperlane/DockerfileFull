# docker/hyperlane/Dockerfile
# syntax=docker/dockerfile:1.4
# Multi-stage build for Hyperlane Rust agents (left-curve fork, branch dango)
#
# Stage 1: Build Rust binaries from the Hyperlane fork
# Stage 2: Create a minimal runtime image with only the required binaries

#########################
#     BASE + TOOLS      #
#########################
FROM rust:1.89.0 AS base

# Install necessary build tools and musl support (same as upstream Hyperlane)
RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools clang git && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo install --locked sccache

# Configure sccache for faster builds
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache

#########################
#        BUILDER        #
#########################
FROM base AS builder

# Arguments for selecting the Hyperlane fork and branch
# These can be overridden from the CI workflow
ARG HYPERLANE_REPO=https://github.com/left-curve/hyperlane-monorepo.git
ARG HYPERLANE_REF=dango

# Clone the Hyperlane fork repository
WORKDIR /usr/src
RUN git clone --depth 1 --branch "${HYPERLANE_REF}" "${HYPERLANE_REPO}" hyperlane

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
    chmod 777 /app && \
    mkdir -p /usr/share/hyperlane && chmod 1000 /usr/share/hyperlane && \
    mkdir -p /data && chown -R 1000 /data

# Run as non-root user for improved security
USER 1000

# Use tini as the init process (same as Hyperlane upstream)
ENTRYPOINT ["tini", "--"]

# Default command (can be overridden with docker run ...)
CMD ["./validator"]
