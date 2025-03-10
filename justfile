set positional-arguments

# List available recipes
default:
  @just --list

# Delete all local git branches except for main
clean-branches:
  git branch | grep -v "main" | xargs git branch -D

# Create a multi-arch Docker builder
docker-create-builder name:
  docker buildx create \
    --name $1 \
    --platform linux/amd64,linux/arm64 \
    --driver docker-container \
    --bootstrap \
    --use

# ------------------------------------ Rust ------------------------------------

# Compile and install the Dango node software
install:
  cargo install --path dango/cli --locked

# Run tests
test:
  RUST_BACKTRACE=1 cargo test --all-features

# Perform linting
lint:
  cargo clippy --bins --tests --benches --examples --all-features --all-targets

# Perform formatting
fmt:
  cargo +nightly fmt --all

# Update wasm artifacts used in tests
testdata:
  cp -v artifacts/grug_{mock_*,tester}.wasm grug/vm/wasm/testdata/

# Build the Left Curve Book
book:
  mdbook build --open

# --------------------------------- Optimizer ----------------------------------

OPTIMIZER_NAME := "leftcurve/optimizer"
OPTIMIZER_VERSION := "0.1.1"

# Build and publish optimizer Docker image
docker-build-optimizer:
  docker buildx build \
    --push \
    --platform linux/amd64,linux/arm64 \
    --tag {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}} \
    --target optimizer \
    docker/optimizer

# Compile and optimize contracts
optimize:
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}
