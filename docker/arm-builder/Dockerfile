FROM debian:bookworm

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

# Set PKG_CONFIG_PATH for aarch64
ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
