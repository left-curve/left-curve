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

# ----------------------------------- Devnet -----------------------------------

DEVNET_NAME := "leftcurve/devnet"
DEVNET_VERSION := "0.2.0"
DEVNET_CHAIN_ID := "dev-2"
DEVNET_GENESIS_TIME := "2024-10-12T00:00:00.000000000Z"

# Build and publish devnet Docker image
docker-build-devnet:
  docker buildx build \
    --push \
    --platform linux/amd64,linux/arm64 \
    --tag {{DEVNET_NAME}}:{{DEVNET_VERSION}} \
    --build-arg CHAIN_ID={{DEVNET_CHAIN_ID}} \
    --build-arg GENESIS_TIME={{DEVNET_GENESIS_TIME}} \
    docker/devnet

# Start a devnet from genesis
start-devnet:
  docker run --name {{DEVNET_CHAIN_ID}} -it -p 26657:26657 -p 26656:26656 {{DEVNET_NAME}}:{{DEVNET_VERSION}}

# Restart a devnet that have been previous stopped
restart-devnet:
  docker start -i {{DEVNET_CHAIN_ID}}

# Remove a devnet
remove-devnet:
  docker rm -f {{DEVNET_CHAIN_ID}}
