FROM rust:1.85.0-slim

# Install dependencies.
RUN apt update && apt install -y clang git libssl-dev pkg-config

# Version of the left-curve monorepo.
ARG LEFT_CURVE_GIT_COMMIT=main
ENV LEFT_CURVE_GIT_COMMIT=$LEFT_CURVE_GIT_COMMIT

# Download dango source code, build, then delete.
RUN echo "Cloning commit: $LEFT_CURVE_GIT_COMMIT" \
  && git init left-curve \
  && cd left-curve \
  && git remote add origin https://github.com/left-curve/left-curve.git \
  && git fetch --depth 1 origin $LEFT_CURVE_GIT_COMMIT \
  && git checkout FETCH_HEAD \
  && cargo install --path dango/cli \
  && cd .. \
  && rm -rf left-curve

# Expose the indexer httpd (8080) and Tendermint ABCI (26658) ports.
EXPOSE 8080 26658

# Run dango.
# This assumes that the `app.toml` file has been mounted to `/root/.dango/config/app.toml`.
CMD ["dango", "start"]
