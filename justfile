# List available recipes
default:
  @just --list

# Delete all git branches except for main
git-clean:
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

# TODO: add platform variants (x86_64 or arm64)

# Build optimizer Docker image
optimizer-build:
  docker build -t {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}} --target optimizer --load docker/optimizer

# Publish optimizer Docker image
optimizer-publish:
  docker push {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}

# Compile and optimize contracts
optimize:
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}

# ----------------------------------- Devnet -----------------------------------

# TODO...
