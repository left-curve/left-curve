FROM rust:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    gcc \
    binutils \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    curl \
    ca-certificates \
    git && \
    rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
