# Dango Core

An execution environment for blockchains.

## Contents

| Crate                                  | Description                                                                |
| -------------------------------------- | -------------------------------------------------------------------------- |
| [app](./app/)                          | State machine transition logics and [Tendermint ABCI][abci] implementation |
| [crypto](./crypto/)                    | Cryptography functionalities                                               |
| [db/disk](./db/disk/)                  | An on-disk, persisted DB backend                                           |
| [db/memory](./db/memory/)              | An in-memory, ephemeral DB backend; used for testing                       |
| [ffi](./ffi/)                          | Helpers for building or interacting with [FFI][ffi]                        |
| [macros](./macros/)                    | Procedural macros for reducing boilerplates in contract developments       |
| [math](./math/)                        | Math primitives                                                            |
| [jellyfish-merkle](./jellyfish-merkl/) | [Jellyfish Merkle Tree][jmt] (JMT) implementation                          |
| [storage](./storage/)                  | Abstractions over key-value stores                                         |
| [types](./types/)                      | Types, traits, and some helper functions                                   |
| [vm/rust](./vm/rust/)                  | A VM that runs native Rust codes; used for testing                         |
| [vm/wasm](./vm/wasm/)                  | A VM that runs WebAssembly byte codes                                      |

[abci]: https://github.com/tendermint/tendermint/tree/main/spec/abci
[ffi]: https://en.wikipedia.org/wiki/Foreign_function_interface
[jmt]: https://developers.diem.com/docs/technical-papers/jellyfish-merkle-tree-paper/
