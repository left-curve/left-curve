# Wasm host example

An demo on how to call Wasm modules, using Rust and [Wasmi](https://github.com/paritytech/wasmi).

## How to use

Build the Wasm binary:

```bash
cargo build --release --target wasm32-unknown-unknown -p guest
```

The demos are presented as tests in `host/src/lib.rs`. To run these tests:

```bash
cargo test -p host
```
