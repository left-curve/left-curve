FROM rust:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y \
      crossbuild-essential-arm64 \
      libssl-dev:arm64 \
      clang \
      libclang-dev \
      llvm-dev \
      gcc-aarch64-linux-gnu \
      binutils-aarch64-linux-gnu \
      pkg-config \
      curl \
      ca-certificates \
      git \
      gcc-aarch64-linux-gnu && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu

RUN update-alternatives --install /usr/bin/cc cc /usr/bin/aarch64-linux-gnu-gcc 100

ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV OPENSSL_DIR=/usr
ENV OPENSSL_LIB_DIR=/usr/lib/aarch64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include/aarch64-linux-gnu
