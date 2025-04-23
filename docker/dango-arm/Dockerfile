FROM rust:slim

LABEL org.opencontainers.image.description="Dango docker image"

RUN apt-get update && \
    apt-get install -y libssl-dev pkg-config && \
    rm -rf /var/lib/apt/lists/*

COPY target/aarch64-unknown-linux-gnu/release/dango /usr/local/bin/dango

# Expose the indexer httpd (8080) and Tendermint ABCI (26658) ports.
EXPOSE 8080 26658

# Run dango.
# This assumes that the `app.toml` file has been mounted to `/root/.dango/config/app.toml`.
CMD ["dango", "start"]
