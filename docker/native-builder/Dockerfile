FROM rust:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    curl \
    ca-certificates \
    git && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu
RUN rustup target add x86_64-unknown-linux-gnu

ARG DEBIAN_ARCH=x86_64-linux-gnu
ENV PKG_CONFIG_PATH=/usr/lib/${DEBIAN_ARCH}/pkgconfig
