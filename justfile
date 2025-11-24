set positional-arguments

# List available recipes
default:
  @just --list

# ------------------------------------ Git -------------------------------------

# Sync the `main` branch with the origin
git-fetch-main:
  git fetch origin main:main

# Delete all local git branches except for main
git-clear-branches:
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

# Check whether the code compiles
check:
  cargo check --bins --tests --benches --examples --all-features --all-targets

# Perform linting
lint:
  cargo clippy --bins --tests --benches --examples --all-features --all-targets -- -D warnings

# Perform formatting
fmt:
  cargo +nightly fmt --all

# Build schema
build-graphql-schema:
  cargo run -p dango-httpd build_graphql_schema -- \
    ./indexer/client/src/schemas/schema.graphql

# Build the Dango Book
book:
  mdbook build --open

# Update CometBFT genesis files
update-genesis:
  cargo run -p dango-scripts --example build_genesis -- \
    networks/localdango/configs/cometbft/config/genesis.json \
    deploy/roles/full-app/templates/config/cometbft/genesis.json

# Update wasm artifacts used in tests
update-testdata:
  cp -v artifacts/grug_{mock_*,tester}.wasm grug/vm/wasm/testdata/

# ---------------------------------- Frontend ----------------------------------

run-website:
  pnpm i
  pnpm dev:portal-web

# --------------------------------- Optimizer ----------------------------------

OPTIMIZER_NAME := "leftcurve/bob-arm64"
OPTIMIZER_VERSION := "0.2.0"

# Compile and optimize contracts
optimize:
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    {{OPTIMIZER_NAME}}:{{OPTIMIZER_VERSION}}

# ----------------------------------- Debug ------------------------------------

check-candles:
  INDEXER__CLICKHOUSE__URL="http://localhost:8123" \
    INDEXER__DATABASE__URL=postgres://postgres@localhost:5432/grug_dev \
    INDEXER__CLICKHOUSE__DATABASE=testnet_dango_production \
    INDEXER__CLICKHOUSE__PASSWORD=${CLICKHOUSE_PASSWORD} \
    RUST_LOG=info \
    cargo run -p dango-cli indexer --home networks/localdango/configs/dango/ check-candles
