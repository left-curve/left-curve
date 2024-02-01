# List available recipes
default:
  @just --list

# Compile and install cwd and cwcli executables
install:
  cargo install --path bin/cwd && cargo install --path bin/cwcli

# Run tests
test:
  cargo test --all-features --all-targets

# Perform linting
lint:
  cargo +nightly clippy --bins --tests --benches --examples --all-features --all-targets

# Check for unused dependencies (https://github.com/est31/cargo-udeps)
udeps:
  cargo +nightly udeps --bins --tests --benches --examples --all-features --all-targets

# Compile and optimize contracts (https://github.com/CosmWasm/rust-optimizer)
optimize:
  if [[ $(uname -m) =~ "arm64" ]]; then \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/optimizer-arm64:0.15.0; else \
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    cosmwasm/optimizer:0.15.0; fi
