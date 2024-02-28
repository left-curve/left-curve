# Cosmos Wasm Daemon

A blockchain framework inspired by [CosmWasm](https://cosmwasm.com/).

## How to use

Prerequisites:

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [Just](https://just.systems/man/en/)
- [Docker](https://docs.docker.com/engine/install/)

Install the **cwd** and **cwcli** command line software:

```shell
just install
```

Run tests:

```shell
just test
```

Lint the code:

```shell
just lint
```

Compile and optimize smart contracts:

```shell
just optimize
```

## Toolchain

The developer tooling we use for this project is listed below. Finding the right tools is important for productivity but can take time. We hope you will find this useful:

| tool            | Rust         | TypeScript                                            |
| --------------- | ------------ | ----------------------------------------------------- |
| package manager | cargo        | yarn (v4)                                             |
| bundler         | cargo build  | [tsup](https://www.npmjs.com/package/tsup)            |
| tester          | cargo test   | [vitest](https://www.npmjs.com/package/vitest)        |
| linter          | cargo clippy | [biome](https://www.npmjs.com/package/@biomejs/biome) |
| formatter       | cargo fmt    | [biome](https://www.npmjs.com/package/@biomejs/biome) |
| documentation   | cargo doc    | [typedoc](https://www.npmjs.com/package/typedoc)      |

## License

TBD
