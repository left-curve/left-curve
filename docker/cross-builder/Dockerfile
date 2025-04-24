FROM debian:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      dpkg --add-architecture arm64 && \
      apt-get update && \
      apt-get install -y \
        crossbuild-essential-arm64 \
        libssl-dev:arm64 \
        gcc-aarch64-linux-gnu \
        binutils-aarch64-linux-gnu; \
    else \
      apt-get update && \
      apt-get install -y \
        build-essential \
        libssl-dev \
        gcc \
        binutils; \
    fi && \
    apt-get install -y \
      clang \
      libclang-dev \
      llvm-dev \
      pkg-config \
      curl \
      ca-certificates \
      git && \
    rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_PATH=/usr/lib/${TARGETARCH}-linux-gnu/pkgconfig
