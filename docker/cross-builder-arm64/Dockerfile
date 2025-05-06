FROM rust:bookworm

LABEL org.opencontainers.image.description="Dango cross builder"
LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN dpkg --add-architecture arm64 && \
    apt-get update && apt-get install -y \
    crossbuild-essential-arm64 \
    libssl-dev:arm64 \
    gcc-aarch64-linux-gnu \
    binutils-aarch64-linux-gnu \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    curl \
    ca-certificates \
    git && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu

ENV OPENSSL_DIR=/usr
ENV OPENSSL_INCLUDE_DIR=/usr/include/aarch64-linux-gnu
ENV OPENSSL_LIB_DIR=/usr/lib/aarch64-linux-gnu
ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_AR=aarch64-linux-gnu-ar
