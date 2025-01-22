<div align="center">
  <img src="book/left-curve.svg" width="150" styles="">
</div>

# Left Curve

This is a [monorepo](https://en.wikipedia.org/wiki/Monorepo) containing a number of [Left Curve Software](https://x.com/leftCurveSoft) products:

| Name              | Language   | Description                                 |
| ----------------- | ---------- | ------------------------------------------- |
| [dango](./dango/) | Rust       | A suite of DeFi application smart contracts |
| [grug](./grug/)   | Rust       | An execution environment for blockchains    |
| [sdk](./sdk/)     | TypeScript | An SDK for interacting with Grug chains     |
| [ui](./ui/)       | TypeScript | A web interface for accessing Dango         |

## How to use

Prerequisites:

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [Just](https://just.systems/man/en/)
- [Docker](https://docs.docker.com/engine/install/)
- [Node.js](https://nodejs.org/en/download/) [version 18+](https://github.com/left-curve/left-curve/blob/main/sdk/README.md?plain=1#L62)
- [pnpm](https://pnpm.io/)
- [Forge](https://book.getfoundry.sh/getting-started/installation)

We use [VS Code](https://code.visualstudio.com/) as text editor with the following plugins:

- [Biomejs](https://marketplace.visualstudio.com/items?itemName=biomejs.biome)
- [Code Spell Checker](https://marketplace.visualstudio.com/items?itemName=streetsidesoftware.code-spell-checker)
- [Dependi](https://marketplace.visualstudio.com/items?itemName=fill-labs.dependi)
- [EditorConfig](https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig)
- [Error Lens](https://marketplace.visualstudio.com/items?itemName=usernamehw.errorlens)
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
- [LaTeX Workshop](https://marketplace.visualstudio.com/items?itemName=James-Yu.latex-workshop)
- [Markdown All in One](https://marketplace.visualstudio.com/items?itemName=yzhang.markdown-all-in-one)
- [markdownlint](https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [solidity](https://marketplace.visualstudio.com/items?itemName=JuanBlanco.solidity)
- [Todo Tree](https://marketplace.visualstudio.com/items?itemName=Gruntfuggly.todo-tree)
- [Trailing Spaces](https://marketplace.visualstudio.com/items?itemName=shardulm94.trailing-spaces)

### Rust

Install the `grug` command line software:

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

### TypeScript

Before running any command is necessary to run:

```shell
pnpm install
```

Start the development mode for all packages in the `sdk/packages`:

```shell
pnpm dev:pkg
```

Start the development mode for the app located in the `ui/portal` directory:

```shell
pnpm dev:portal
```

Build the SDK:

```shell
pnpm build:pkg
```

Build both the SDK and the Portal App:

```shell
pnpm build:portal
```

Run tests:

```shell
pnpm test
```

Run linter:

```shell
pnpm lint
```

Generate documentation:

```shell
pnpm doc
```

Storybook:

```shell
pnpm storybook
```

## Solidity

Install dependencies:

```shell
forge soldeer install
```

Install dependencies of dependencies (this isn't done automatically in the previous step):

```shell
cd dependencies/hyperlane-monorepo-0.0.0
yarn install
```

Compile contracts:

```shell
forge build
```

Run tests:

```shell
forge test
```

## Docs

Install dependencies:

```shell
cargo install mdbook
cargo install mdbook-katex
```

Generate docs:

```shell
mdbook build
```

## Acknowledgement

> TODO

## License

> TBD
