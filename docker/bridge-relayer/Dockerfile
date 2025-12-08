FROM debian:bookworm-slim

LABEL org.opencontainers.image.description="Bridge relayer HTTPD docker image"
LABEL org.opencontainers.image.revision=$GIT_COMMIT
LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

ARG TARGET_ARCH
ARG GIT_COMMIT
ENV GIT_COMMIT=$GIT_COMMIT

RUN apt-get update && \
    apt-get install -y libssl-dev pkg-config libsqlite3-0 ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY target/${TARGET_ARCH}/release/bridge-relayer-httpd /usr/local/bin/
WORKDIR /app

# Expose the indexer httpd (8080), and indexer metrics (8081).
EXPOSE 8080 8081

# Run bridge relayer httpd.
CMD ["bridge-relayer-httpd"]
