FROM golang:1.24-bookworm AS builder

LABEL org.opencontainers.image.description="CometBFT builder for Dango"
LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

# Install dependencies.
RUN apt update && apt install git make

# Git tag of the cometbft repo.
ARG COMETBFT_GIT_TAG="v0.38.17"
ENV COMETBFT_GIT_TAG=$COMETBFT_GIT_TAG

# Download cometbft source code, build, then delete.
RUN git clone --branch $COMETBFT_GIT_TAG --depth 1 https://github.com/cometbft/cometbft.git \
  && cd cometbft \
  && make install

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.description="CometBFT runtime for Dango"

COPY --from=builder /go/bin/cometbft /usr/local/bin/

# Expose the Tendermint p2p (26656), RPC (26657), and ABCI (26658) ports.
EXPOSE 26656 26657 26658

# Run cometbft.
# This assume the config folder has been mounted to `/root/.cometbft/config`.
CMD ["cometbft", "start"]
