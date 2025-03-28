FROM rust:1.85.0-slim

# Install dependencies.
RUN apt update && apt install -y clang git libssl-dev pkg-config

# Version of the left-curve monorepo.
ARG LEFT_CURVE_GIT_COMMIT="76e80c5"

# Download dango source code, build, then delete.
RUN git clone https://github.com/left-curve/left-curve.git \
  && cd left-curve \
  && git checkout $LEFT_CURVE_GIT_COMMIT \
  && cargo install --path dango/cli \
  && cd .. \
  && rm -rf left-curve

# Expose the indexer httpd (8080) and Tendermint ABCI (26658) ports.
EXPOSE 8080 26658

# Run dango.
# This assumes that the `app.toml` file has been mounted to `/root/.dango/config/app.toml`.
CMD ["dango", "start"]
