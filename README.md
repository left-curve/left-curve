# Cosmos Wasm Daemon

A blockchain framework inspired by [CosmWasm](https://cosmwasm.com/).

## How to use

Prerequisites:

- [Rust](https://rustup.rs/) and `wasm32-unknown-unknown` target
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

Check for unused dependencies:

```shell
just udeps
```

Compile and optimize smart contracts:

```shell
just optimize
```

## License

TBD
