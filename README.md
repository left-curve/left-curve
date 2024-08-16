## Interface

You can find more info in our [documentation](https://ui-doc.pages.dev/).

## Packages

#### `/apps`

| App        | Description |
| ---------- | ----------- |
| [SuperApp] | -           |

#### `/packages`

| Package                           | Description                                                                                                                                                 |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`applets`](./packages/applets)   | React components, hooks, providers and others that help to build [SuperApp]. It includes a [Storybook] server.                                              |
| [`clients`](./packages/clients)   | -                                                                                                                                                           |
| [`config`](./packages/config)     | Common configurations for tsconfig, tsup, biome, typedoc and tailwind                                                                                       |
| [`crypto`](./packages/crypto)     | It includes various cryptographic functions and utilities for encryption, decryption, hashing, and more.                                                    |
| [`encoding`](./packages/encoding) | Encoding helpers that are used across packages and assist in encoding and decoding data in various formats                                                  |
| [`types`](./packages/types)       | It contains various type definition used across the codebase. These types help ensure type safety and improve code readbility.                              |
| [`utils`](./packages/utils)       | Its a collection of utility functions that are used across the project. These utilities are designed to simplify common tasks and improve code reusability. |

## Supported JS environments
Packages in the workspace are compiled to JavaScript ES2021, targeting the latest ECMAScript standard, and support both ESM and CJS module formats.

1. Node.js 18+
2. Modern browsers (Chromium/Firefox/Safari)
3. Browser extensions (Chromium/Firefox)

## Development

See [Hacking.md]

## Acknowledgement

This project draws inspiration from and follows some architecture design of [Viem]. Several concepts and ideas are directly adapted from their codebase, which greatly influenced this project.

Additionally, we would like to acknowledge [Cosmjs] for providing foundational code and tools that contributed to key parts of this project.

We are grateful to both [Viem] and [Cosmjs] for their open-source contributions and the community support they provide.

## License

TBD

[Storybook]: https://storybook.js.org/
[SuperApp]: ./apps/superapp
[Hacking.md]: ./HACKING.md
[Viem]: https://github.com/wevm/viem
[Cosmjs]: https://github.com/cosmos/cosmjs
