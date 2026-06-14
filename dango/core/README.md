# Grug

An execution environment for blockchains.

[Whitepaper][grug]

## Contents

| Crate                                         | Description                                                                |
| --------------------------------------------- | -------------------------------------------------------------------------- |
| [app](./crates/app)                           | State machine transition logics and [Tendermint ABCI][abci] implementation |
| [client](./crates/client/)                    | HTTP client for interacting with Grug via Tendermint RPC                   |
| [crypto](./crates/crypto)                     | Cryptography functionalities                                               |
| [db/disk](./crates/db/disk)                   | An on-disk, persisted DB backend                                           |
| [db/memory](./crates/db/memory)               | An in-memory, ephemeral DB backend; used for testing                       |
| [ffi](./crates/ffi)                           | Helpers for building or interacting with [FFI][ffi]                        |
| [macros](./crates/macros)                     | Procedural macros for reducing boilerplates in contract developments       |
| [jellyfish-merkle](./crates/jellyfish-merkle) | [Jellyfish Merkle Tree][jmt] (JMT) implementation                          |
| [std](./crates/std)                           | A "meta crate" the re-exports contents of other crates                     |
| [storage](./crates/storage)                   | Abstractions over key-value stores                                         |
| [testing](./crates/testing)                   | Testing utilities                                                          |
| [types](./crates/types)                       | Types, traits, and some helper functions                                   |
| [vm/rust](./crates/vm/rust)                   | A VM that runs native Rust codes; used for testing                         |
| [vm/wasm](./crates/vm/wasm)                   | A VM that runs WebAssembly byte codes                                      |

[abci]:   https://github.com/tendermint/tendermint/tree/main/spec/abci
[grug]:   https://leftcurve.software/grug.html
[ffi]:    https://en.wikipedia.org/wiki/Foreign_function_interface
[jmt]:    https://developers.diem.com/docs/technical-papers/jellyfish-merkle-tree-paper/
