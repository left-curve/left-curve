FROM rust:slim AS builder

LABEL org.opencontainers.image.description="Dango build stage"

# Install dependencies.
RUN apt update && apt install -y clang git libssl-dev pkg-config

# Download the crates.io index using the new sparse protocol to improve performance
# and avoid OOM in the build.
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Version of the left-curve monorepo.
ARG GIT_COMMIT=main
ENV COMMIT=$GIT_COMMIT

# Download dango source code, build, then delete.
RUN echo "Cloning commit: $GIT_COMMIT" \
  && git init left-curve \
  && cd left-curve \
  && git remote add origin https://github.com/left-curve/left-curve.git \
  && git fetch --depth 1 origin $GIT_COMMIT \
  && git checkout FETCH_HEAD \
  && cp dango/cli/src/git_info.rs dango/cli/src/git_info.rs.in \
  && sed "s/{{GIT_COMMIT}}/${GIT_COMMIT}/" dango/cli/src/git_info.rs.in > dango/cli/src/git_info.rs \
  && rm dango/cli/src/git_info.rs.in \
  && cargo install --path dango/cli

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.description="Dango runtime image"

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    # apt-get install -y libssl-dev pkg-config && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /left-curve/target/release/dango /usr/local/bin/dango

EXPOSE 8080 26658

# Run dango.
# This assumes that the `app.toml` file has been mounted to `/root/.dango/config/app.toml`.
CMD ["dango", "start"]
