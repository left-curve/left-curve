<div align="center">
  <img src="book/left-curve.svg" width="150" styles="">
</div>

# Left Curve

This is a [monorepo](https://en.wikipedia.org/wiki/Monorepo) containing a number of [Left Curve Software](https://x.com/leftCurveSoft) products:

| Name                      | Language   | Description                                                                                      |
| ------------------------- | ---------- | ------------------------------------------------------------------------------------------------ |
| [book](./book/)           | Markdown   | documentation, created with [mdBook](https://rust-lang.github.io/mdBook/)                        |
| [dango](./dango/)         | Rust       | a suite of DeFi application smart contracts                                                      |
| [grug](./grug/)           | Rust       | an execution environment for blockchains                                                         |
| [hyperlane](./hyperlane/) | Rust       | implementation of the [Hyperlane](https://hyperlane.xyz/) cross-chain messaging protocol in Grug |
| [indexer](./indexer/)     | Rust       | indexer and server infrastructure                                                                |
| [sdk](./sdk/)             | TypeScript | an SDK for interacting with Grug chains                                                          |
| [ui](./ui/)               | TypeScript | a web interface for accessing Dango                                                              |

## How to use

Prerequisites:

- [Rust](https://rustup.rs/) 1.80+
- [Node.js](https://nodejs.org/en/download/) 21.0+
- [pnpm](https://pnpm.io/)
- [Just](https://just.systems/man/en/)
- [Docker](https://docs.docker.com/engine/install/)

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

Start the development mode for dango and grug sdk:

```shell
pnpm dev:sdk
```

Start the development mode for the app located in the `ui/portal/web` directory:

```shell
pnpm dev:portal-web
```

Build Grug and Dango SDK:

```shell
pnpm build:sdk
```

Build Portal Website:

```shell
pnpm build:portal-web
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

Generate translations:

```shell
pnpm machine-translate
```

Storybook:

```shell
pnpm storybook
```

## Book

Install dependencies:

```shell
cargo install mdbook
cargo install mdbook-katex
```

Generate book:

```shell
mdbook build
```

## Copyright

© 2024-2025 Left Curve Software. All rights reserved.

This repository and all its contents, including but not limited to source code, documentation, specifications, and associated materials (collectively, the "Software") are proprietary to Left Curve Software Limited (the "Company") and are provided for informational purposes only. No license, express or implied, is granted. No part of the Software may be modified, forked, distributed, sublicensed, or used in any manner, for commercial or non-commercial purpose, without express written permission of the Company.
