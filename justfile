set positional-arguments

# List available recipes
default:
  @just --list

# Delete all local git branches except for main
clean-branches:
  git branch | grep -v "main" | xargs git branch -D

# ------------------------------------ Rust ------------------------------------

# Compile and install the Dango node software
install:
  cargo install --path dango/cli --locked

# Run all tests
test:
  RUST_BACKTRACE=1 cargo test --all-features -- --nocapture

# Run grug tests
test-grug:
  RUST_BACKTRACE=1 cargo test --all-features -p grug-testing -- --nocapture

# Run dango tests
test-dango:
  RUST_BACKTRACE=1 cargo test --all-features -p dango-testing -- --nocapture

# Run indexer tests
test-indexer:
  RUST_BACKTRACE=1 cargo test --all-features -p indexer-testing -- --nocapture

# Perform linting
lint:
  cargo clippy --bins --tests --benches --examples --all-features --all-targets

# Perform formatting
fmt:
  cargo +nightly fmt --all

# Build schema
build-graphql-schema:
  cargo run -p dango-httpd build_graphql_schema -- \
    ./indexer/client/src/schemas/schema.graphql

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

# ------------------------------- Cross Builder --------------------------------

docker-build-builder-images:
  docker buildx bake --push

  # Combine the two into a manifest
  docker manifest create ghcr.io/left-curve/left-curve/native-builder:latest \
    --amend ghcr.io/left-curve/left-curve/native-builder:amd64 \
    --amend ghcr.io/left-curve/left-curve/native-builder:arm64

  # Push the manifest
  docker manifest push ghcr.io/left-curve/left-curve/native-builder:latest
