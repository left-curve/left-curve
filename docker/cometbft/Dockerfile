FROM golang:1.24-alpine

# Install dependencies.
RUN apk update && apk add git make

# Git tag of the cometbft repo.
ARG COMETBFT_GIT_TAG="v0.38.17"

# Download cometbft source code, build, then delete.
RUN git clone --branch $COMETBFT_GIT_TAG --depth 1 https://github.com/cometbft/cometbft.git \
  && cd cometbft \
  && make install \
  && cd .. \
  && rm -rf cometbft

# Expose the Tendermint p2p (26656), RPC (26657), and ABCI (26658) ports.
EXPOSE 26656 26657 26658

# Run cometbft.
# This assume the config folder has been mounted to `/root/.cometbft/config`.
CMD ["cometbft", "start"]
