FROM rust:bookworm

LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

RUN dpkg --add-architecture amd64 && \
    apt-get update && apt-get install -y \
    crossbuild-essential-amd64 \
    libssl-dev:amd64 \
    libsqlite3-dev:amd64 \
    librocksdb-dev:amd64 \
    libsnappy-dev:amd64 \
    zlib1g-dev:amd64 \
    libbz2-dev:amd64 \
    liblz4-dev:amd64 \
    libzstd-dev:amd64 \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu \
    binutils-x86-64-linux-gnu \
    pkg-config \
    curl \
    ca-certificates \
    git && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-gnu

RUN update-alternatives --install /usr/bin/cc cc /usr/bin/x86_64-linux-gnu-gcc 100 && \
    update-alternatives --install /usr/bin/c++ c++ /usr/bin/x86_64-linux-gnu-g++ 100 && \
    update-alternatives --install /usr/bin/gcc gcc /usr/bin/x86_64-linux-gnu-gcc 100 && \
    update-alternatives --install /usr/bin/g++ g++ /usr/bin/x86_64-linux-gnu-g++ 100

ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++
ENV AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar
ENV LD_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ld
ENV CC=x86_64-linux-gnu-gcc
ENV CXX=x86_64-linux-gnu-g++
ENV LD=x86_64-linux-gnu-ld
ENV OPENSSL_DIR=/usr
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include/x86_64-linux-gnu
ENV SQLITE3_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV SQLITE3_INCLUDE_DIR=/usr/include
