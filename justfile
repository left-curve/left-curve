# List available recipes
default:
  @just --list

# Delete all git branches except for main
clean-branches:
  git branch | grep -v "main" | xargs git branch -D

# ------------------------------------ Rust ------------------------------------

# Compile and install the Grug node software
install:
  cargo install --path bin

# Run tests
test:
  RUST_BACKTRACE=1 cargo test --all-features

# Perform linting
lint:
  cargo clippy --bins --tests --benches --examples --all-features --all-targets

# Perform formatting
fmt:
  cargo +nightly fmt --all

# Update data used for wasmvm tests
testdata:
  cp -v artifacts/grug_{mock_*,tester}.wasm grug/vm-wasm/testdata/

# --------------------------------- Optimizer ----------------------------------

OPTIMIZER_NAME := "leftcurve/optimizer"
OPTIMIZER_VERSION := "0.1.0"

# Build optimizer Docker image for x86_64
docker-build-optimizer-x86:
  docker build --pull --load --platform linux/amd64 \
    -t {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}} --target optimizer docker/optimizer

# Build optimizer Docker image for arm64
docker-build-optimizer-arm64:
  docker build --pull --load --platform linux/arm64/v8 \
    -t {{OPTIMIZER_NAME}}-arm64:{{OPTIMIZER_VERSION}} --target optimizer docker/optimizer

# Publish optimizer Docker image for x86_64
docker-publish-optimizer-x86:
  docker push {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}

# Publish optimizer Docker image for arm64
docker-publish-optimizer-arm64:
  docker push {{OPTIMIZER_NAME}}-arm64:{{OPTIMIZER_VERSION}}

# Compile and optimize contracts
optimize:
  if [[ $(uname -m) =~ "arm64" ]]; then \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    {{OPTIMIZER_NAME}}-arm64:{{OPTIMIZER_VERSION}}; else \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}; fi

# ----------------------------------- Devnet -----------------------------------

DEVNET_NAME := "leftcurve/devnet"
DEVNET_VERSION := "0.1.0"

# Build devnet Docker image
#
# Note: For this to work, it may be necessary to create a custom builder with
# the docker-container driver:
# $ docker buildx create --name leftcurve --use
# $ docker buildx inspect leftcurve --bootstrap
docker-build-devnet:
  docker buildx build --platform linux/amd64,linux/arm64 \
    -t {{DEVNET_NAME}}:{{DEVNET_VERSION}} --target devnet docker/devnet

# Publish devnet Docker image
docker-publish-devnet:
  docker push {{DEVNET_NAME}}:{{DEVNET_VERSION}}

# Run devnet
devnet:
  if [[ $(uname -m) =~ "arm64" ]]; then \
  docker run -it -p 26657:26657 -p 26656:26656 {{DEVNET_NAME}}-arm64:{{DEVNET_VERSION}}; else \
  docker run -it -p 26657:26657 -p 26656:26656 {{DEVNET_NAME}}:{{DEVNET_VERSION}}; fi
