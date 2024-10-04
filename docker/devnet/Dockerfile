# ---------------------------------- cometbft ----------------------------------

FROM golang:1.23.2-alpine AS cometbft

ARG COMETBFT_GIT_TAG="v0.38.12"

# Install build dependencies
RUN apk update && apk add --no-cache git make

# Download source code
RUN git clone https://github.com/cometbft/cometbft.git \
  && cd cometbft \
  && git checkout $COMETBFT_GIT_TAG

# Compile and install cometbft
RUN cd cometbft \
  && make build \
  && mv build/cometbft /usr/local/bin

# Clean up
RUN rm -rf cometbft

# ------------------------------------ grug ------------------------------------

FROM rust:1.81.0-alpine AS grug

ARG LEFT_CURVE_GIT_COMMIT="879e7d1"

# Download the crates.io index using the new sparse protocol to improve performance
# and avoid OOM in the build.
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Install build dependencies
# musl-dev provides crti.o
# clang-dev and llvm-dev both provide libclang.so, located respectively at:
# - /usr/lib/libclang.so
# - /usr/lib/llvm17/lib/libclang.so
# Can use a specific one by setting the LIBCLANG_PATH environment variable.
# But this isn't necessary here.
RUN apk update && apk add --no-cache clang clang-dev g++ git llvm-dev musl-dev

# Download source code
RUN git clone https://github.com/left-curve/left-curve.git \
  && cd left-curve \
  && git checkout $LEFT_CURVE_GIT_COMMIT

# Compile and install grug
# crt-static must be disabled. See:
# https://github.com/apache/skywalking/issues/10439
# https://skywalking.apache.org/docs/skywalking-php/next/en/setup/service-agent/php-agent/readme/#install
RUN cd left-curve \
  && RUSTFLAGS='-C link-arg=-s -C target-feature=-crt-static' cargo build -p grug-cli --release \
  && mv target/release/grug /usr/local/bin

# Clean up
RUN rm -rf left-curve

# ----------------------------------- devnet -----------------------------------

FROM alpine:3.20 AS devnet

# TODO
