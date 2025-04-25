FROM rust:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN dpkg --add-architecture amd64 && \
    apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    gcc \
    binutils \
    gcc-x86-64-linux-gnu \
    binutils-x86-64-linux-gnu \
    libssl-dev:amd64 \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    curl \
    ca-certificates \
    git && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-gnu

ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV OPENSSL_DIR=/usr
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include/x86_64-linux-gnu
