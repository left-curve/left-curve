FROM debian:bookworm-slim

LABEL org.opencontainers.image.description="Dango docker image"
LABEL org.opencontainers.image.revision=$GIT_COMMIT
LABEL org.opencontainers.image.source="https://github.com/left-curve/left-curve"

ARG TARGET_ARCH
ARG GIT_COMMIT
ENV GIT_COMMIT=$GIT_COMMIT
# Read why it's commented in the cometbft image
# ARG USERNAME=dango
# ENV USERNAME=${USERNAME}

RUN apt-get update && \
    apt-get install -y libssl-dev pkg-config libsqlite3-0 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# RUN useradd --create-home --shell /bin/bash $USERNAME

COPY target/${TARGET_ARCH}/release/dango /usr/local/bin/
# RUN chown $USERNAME:$USERNAME /usr/local/bin/dango

# USER $USERNAME
# WORKDIR /home/$USERNAME
WORKDIR /app

# Expose the indexer httpd (8080) and Tendermint ABCI (26658) ports.
EXPOSE 8080 26658

# Run dango.
# This assumes that the `app.toml` file has been mounted to `/root/.dango/config/app.toml`.
CMD ["dango", "start"]
