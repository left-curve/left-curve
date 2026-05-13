## Dango SDK

TypeScript SDK for the [Dango](https://dango.exchange) ecosystem.

## Packages

| Package                    | Description                                                                      |
| -------------------------- | -------------------------------------------------------------------------------- |
| [`config`](./config)       | Common configurations for tsconfig, tsup, biome, typedoc                         |
| [`dango`](./dango)         | Clients, actions, chains, and transports for interacting with the Dango chain    |
| [`types`](./types)         | TypeScript type definitions used across the ecosystem                            |
| [`encoding`](./encoding)   | Hex, base64, binary, JSON, and UTF-8 encoding utilities                          |
| [`utils`](./utils)         | Decimal math, formatting, assertions, and general utilities                      |
| [`crypto`](./crypto)       | Hash functions, key pairs, WebAuthn, and cryptographic primitives                |

## Supported JS environments

Packages are compiled to JavaScript ES2022 and support both ESM and CJS module formats.

1. Node.js 21+
2. Modern browsers (Chromium/Firefox/Safari)
3. Browser extensions (Chromium/Firefox)

## Acknowledgement

This project draws inspiration from and follows some of the architectural design principles of [Viem], while utilizing foundational code from [Wagmi]. Several concepts and ideas have been directly adapted from their codebase, significantly influencing this project.

Additionally, we would like to acknowledge [CosmJS] for providing essential code and tools that contributed to key aspects of this project.

We are grateful to both the [Wevm] and [Confio] team for their open-source contributions and the valuable support they offer to the community.

## License

TBD

[Dango]: https://dango.exchange
[Wevm]: https://wevm.dev/
[Wagmi]: https://github.com/wevm/wagmi
[Viem]: https://github.com/wevm/viem
[CosmJS]: https://github.com/cosmos/cosmjs
[Confio]: https://confio.gmbh/
