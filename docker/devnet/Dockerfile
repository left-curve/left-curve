FROM leftcurve/optimizer:0.1.0 AS optimizer

FROM rust:1.81.0-alpine

# Target platform the image is being built for.
# Must be either `linux/amd64` or `linux/arm64`.
ARG TARGETPLATFORM

# Chain ID of the devnet.
ARG CHAIN_ID="dev-1"

# Genesis time of the devnet.
ARG GENESIS_TIME="2024-10-06T00:00:00.000000000Z"

# Version of CometBFT.
ARG COMETBFT_VERSION="0.38.12"

# Version of left-curve monorepo with which we will build grug CLI.
ARG LEFT_CURVE_GIT_COMMIT="1f62285"

# Download the crates.io index using the new sparse protocol to improve
# performance and avoid OOM in the build. See:
# https://doc.rust-lang.org/cargo/reference/config.html#registriescrates-ioprotocol
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Copy bob and wasm-opt from optimizer.
COPY --from=optimizer /usr/local/bin/bob /usr/local/bin
COPY --from=optimizer /usr/local/bin/wasm-opt /usr/local/bin

# Setup Rust's wasm target.
RUN rustup target add wasm32-unknown-unknown

# Install build dependencies.
#
# musl-dev provides crti.o
# clang-dev and llvm-dev both provide libclang.so, located respectively at:
# - /usr/lib/libclang.so
# - /usr/lib/llvm17/lib/libclang.so
# Can use a specific one by setting the LIBCLANG_PATH environment variable.
# But this isn't necessary here.
RUN apk update && apk add --no-cache clang clang-dev g++ git llvm-dev musl-dev

# Download pre-built cometbft binary.
RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
  wget -O cometbft.tar.gz https://github.com/cometbft/cometbft/releases/download/v${COMETBFT_VERSION}/cometbft_${COMETBFT_VERSION}_linux_amd64.tar.gz; \
  elif [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
  wget -O cometbft.tar.gz https://github.com/cometbft/cometbft/releases/download/v${COMETBFT_VERSION}/cometbft_${COMETBFT_VERSION}_linux_arm64.tar.gz; \
  else \
  echo "unsupported architecture: $TARGETPLATFORM" && exit 1; \
  fi \
  && tar -xvf cometbft.tar.gz -C /usr/local/bin cometbft \
  && rm cometbft.tar.gz \
  && cometbft version

# Download grug source code.
RUN git clone https://github.com/left-curve/left-curve.git \
  && cd left-curve \
  && git checkout $LEFT_CURVE_GIT_COMMIT

# Compile grug.
#
# Must disable `crt-static`, to dynamically link with C runtime (musl-libc). See:
# - https://github.com/apache/skywalking/issues/10439
# - https://skywalking.apache.org/docs/skywalking-php/next/en/setup/service-agent/php-agent/readme/#install
# Explanation on why this is necessary (in chinese): https://zhuanlan.zhihu.com/p/138109387
RUN cd left-curve \
  && RUSTFLAGS='-C link-arg=-s -C target-feature=-crt-static' cargo build -p grug-cli --release \
  && mv target/release/grug /usr/local/bin \
  && grug --version

# Build and optimize contracts.
RUN cd left-curve \
  && bob \
  && cat artifacts/checksum.txt

# Create genesis file.
# Edit cometbft config such that Tendermint RPC accepts queries from all hosts.
RUN cd left-curve \
  && cometbft init \
  && sed -i 's|laddr = "tcp://127.0.0.1:26657"|laddr = "tcp://0.0.0.0:26657"|g' ~/.cometbft/config/config.toml \
  && cargo run -p dango-genesis --example build_genesis -- ${CHAIN_ID} ${GENESIS_TIME} \
  && cat ~/.cometbft/config/genesis.json

# Delete the left-curve repo.
RUN rm -rf left-curve

# Add run script
ADD run.sh /root

# Expose the ports.
# 26656 - Tendermint p2p
# 26657 - Tendermint RPC
EXPOSE 26656 26657

# Run grug and cometbft
ENTRYPOINT ["/bin/ash", "/root/run.sh"]
