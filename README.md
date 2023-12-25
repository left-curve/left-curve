# Wasm host example

An demo on how to call Wasm modules, using Rust and [Wasmi](https://github.com/paritytech/wasmi).

Code is mostly adapted from [cosmwasm-std](https://github.com/CosmWasm/cosmwasm/tree/main/packages/std) and [vm](https://github.com/CosmWasm/cosmwasm/tree/main/packages/vm) crates, so kudos to their creators.

## Contents

This crate contains three examples:

#### Add

In this Wasm module, the exported function only contains primitive types in its signature. Therefore the host can directly call it without dynamic allocating memory.

#### Greeter

In this Wasm module, the function takes a string and returns a string, which is a non-primitive data.

To call this function, the host needs to:

- dynamically allocate a region in the Wasm memory of appropriate size
- load the input data into this region
- call the function, providing it the memory address pointing to the region

For the Wasm module to return data, the process is similar: put data in the Wasm memory, and return a pointer.

#### Bank

In this example, the module directly reads or writes data to the host state by calling host functions.

## Crates

| Crate            | Descripton                                                         |
| ---------------- | ------------------------------------------------------------------ |
| `guests/add`     | Wasm module for the "add" example                                  |
| `guests/bank`    | Wasm module for the "bank" example                                 |
| `guests/greeter` | Wasm module for the "greeter" example                              |
| `host`           | The host environment                                               |
| `sdk`            | An small SDK containing functions useful for building Wasm modules |

## How to use

To run the "add" example,

```bash
cargo build -p add --target wasm32-unknown-unknown
cargo run -p host --example add
```

For the other two examples, simply change the word in the commands.
