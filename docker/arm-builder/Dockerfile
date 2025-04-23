FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y libssl-dev:arm64 pkg-config && \
    rm -rf /var/lib/apt/lists/*
