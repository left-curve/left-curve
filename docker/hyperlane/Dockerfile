# docker/hyperlane/Dockerfile
# syntax=docker/dockerfile:1.4
# Runtime-only image for Hyperlane Rust agents (left-curve fork, branch dango)
#
# The Rust binaries are built in CI (with cross) and placed under:
#   ./target/<TARGET_ARCH>/release/...
# This Dockerfile only copies those artifacts into a minimal runtime image.

FROM ubuntu:24.04

# Target triple used to locate pre-built binaries under target/<TARGET_ARCH>/release
ARG TARGET_ARCH
# Commit identifier from the Hyperlane fork (injected from the CI workflow)
ARG GIT_COMMIT=unknown
ARG HYPERLANE_DIR=hyperlane

# Useful metadata for OCI registries
LABEL org.opencontainers.image.description="Hyperlane Rust agents (left-curve fork, branch dango)"
LABEL org.opencontainers.image.revision=$GIT_COMMIT
LABEL org.opencontainers.image.source="https://github.com/left-curve/hyperlane-monorepo"

ENV GIT_COMMIT=$GIT_COMMIT

WORKDIR /app

# Copy configuration from the cloned Hyperlane repository (present in the CI workspace)
COPY ${HYPERLANE_DIR}/rust/main/config /app/config
COPY ${HYPERLANE_DIR}/rust/main/app-contexts /app/app-contexts

# Copy pre-built binaries from CI.
COPY target/${TARGET_ARCH}/release/relayer /usr/local/bin/relayer
COPY target/${TARGET_ARCH}/release/validator /usr/local/bin/validator
COPY target/${TARGET_ARCH}/release/scraper /usr/local/bin/scraper


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

# Default to running the relayer (can be overridden with docker run ...)
CMD ["relayer"]
