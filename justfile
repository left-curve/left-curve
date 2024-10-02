# List available recipes
default:
  @just --list

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

# Compile and optimize contracts (https://github.com/CosmWasm/rust-optimizer)
optimize:
  if [[ $(uname -m) =~ "arm64" ]]; then \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    leftcurve/optimizer-arm64:0.17.0-rc.0; else \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    leftcurve/optimizer:0.17.0-rc.0; fi

# Update data used for wasmvm tests
update-test-data:
  cp -v artifacts/grug_mock_*.wasm grug/vm-wasm/testdata/ && \
  cp -v artifacts/grug_tester.wasm grug/vm-wasm/testdata/
