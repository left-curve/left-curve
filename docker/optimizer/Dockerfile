# ------------------------------------ bob -------------------------------------

FROM rust:alpine AS bob

ARG GIT_COMMIT=main
ENV GIT_COMMIT=$GIT_COMMIT

# Download the crates.io index using the new sparse protocol to improve performance
# and avoid OOM in the build.
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Install build dependencies
RUN apk update && apk add --no-cache git musl-dev

# Download dango source code, build, then delete.
RUN echo "Cloning commit: $GIT_COMMIT" \
  && git init left-curve \
  && cd left-curve \
  && git remote add origin https://github.com/left-curve/left-curve.git \
  && git fetch --depth 1 origin $GIT_COMMIT \
  && git checkout FETCH_HEAD \
  && cp grug/app/src/git_info.rs grug/app/src/git_info.rs.in \
  && sed "s/{{GIT_COMMIT}}/${GIT_COMMIT}/" grug/app/src/git_info.rs.in > grug/app/src/git_info.rs \
  && rm grug/app/src/git_info.rs.in \
  && cargo install --path dango/cli \
  && cd .. \
  && rm -rf left-curve

# Compile and install bob
RUN cd left-curve \
  && RUSTFLAGS='-C link-arg=-s' cargo build -p bob --release \
  && mv target/release/bob /usr/local/bin

# Clean up
RUN rm -rf left-curve

# ---------------------------------- wasm-opt ----------------------------------

FROM rust:1.81.0-alpine AS wasm-opt

ARG BINARYEN_GIT_TAG="version_119"

# Install build dependencies
RUN apk update && apk add --no-cache build-base clang cmake git ninja python3

# Download source code
RUN git clone https://github.com/WebAssembly/binaryen.git \
  && cd binaryen \
  && git checkout $BINARYEN_GIT_TAG

# Compile and install wasm-opt
# Adapted from https://github.com/WebAssembly/binaryen/blob/main/.github/workflows/create_release.yml
RUN cd binaryen \
  && git clone --depth 1 https://github.com/google/googletest.git ./third_party/googletest \
  && cmake . -G Ninja -DCMAKE_CXX_FLAGS="-static" -DCMAKE_C_FLAGS="-static" -DCMAKE_BUILD_TYPE=Release -DBUILD_STATIC_LIB=ON \
  && ninja wasm-opt \
  && strip bin/wasm-opt \
  && mv bin/wasm-opt /usr/local/bin

# Clean up
RUN rm -rf binaryen

# --------------------------------- optimizer ----------------------------------

FROM rust:alpine AS optimizer

# Install build dependencies
RUN apk update && apk add --no-cache musl-dev

# Add bob and wasm-opt
COPY --from=bob /usr/local/bin/bob /usr/local/bin
COPY --from=wasm-opt /usr/local/bin/wasm-opt /usr/local/bin

# Setup Rust with Wasm support
RUN rustup target add wasm32-unknown-unknown

# Assume we mount the source code in /code
WORKDIR /code

# Set bob as entry point.
ENTRYPOINT ["/usr/local/bin/bob"]
