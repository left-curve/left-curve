# Wasm host example

An demo on how to call Wasm modules, using Rust and [Wasmi](https://github.com/paritytech/wasmi).

## How to use

Build the Wasm binaries:

```bash
cargo build --release --target wasm32-unknown-unknown -p add -p bank -p greeter
```

The demos are presented as examples in `host/examples`. To run them:

```bash
cargo run -p host --example add
cargo run -p host --example bank
cargo run -p host --example greeter
```
