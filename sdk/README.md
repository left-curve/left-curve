## Grug SDK

You can find more info in our [documentation](https://grug-sdk.pages.dev/).

## Packages

#### `/packages`

| Package                                  | Description                                                                                                                                                 |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`config`](./packages/config)            | Common configurations for tsconfig, tsup, biome, typedoc and tailwind                                                                                       |
| [`connect-kit`](./packages/connect-kit/) | It allows connection to multiple blockchains and wallets, manages accounts, and enables interaction with smart contracts.                                   |
| [`crypto`](./packages/crypto)            | It includes various cryptographic functions and utilities for encryption, decryption, hashing, and more.                                                    |
| [`encoding`](./packages/encoding)        | Encoding helpers that are used across packages and assist in encoding and decoding data in various formats                                                  |
| [`react`](./packages/react)              | It wrap connect-kit actions into react-hooks and wrap the state in a react provider with hydration for ssr                                                  |
| [`sdk`](./packages/sdk)                  | The SDK package provides a set of tools and utilities to interact with [Grug] execution environment                                                         |
| [`types`](./packages/types)              | It contains various type definition used across the codebase. These types help ensure type safety and improve code readbility.                              |
| [`utils`](./packages/utils)              | Its a collection of utility functions that are used across the project. These utilities are designed to simplify common tasks and improve code reusability. |

## Module Graph

```mermaid
%%{
  init: {
  'theme': 'base',
  'themeVariables': {"primaryTextColor":"#fff","primaryColor":"#5a4f7c","lineColor":"#f5a623" }
  }
}%%
stateDiagram-v2
    @left-curve/config --> @left-curve/types
    @left-curve/config --> @left-curve/utils
    @left-curve/types --> @left-curve/utils
    @left-curve/config --> @left-curve/sdk
    @left-curve/types --> @left-curve/sdk
    @left-curve/utils --> @left-curve/sdk
    @left-curve/crypto --> @left-curve/sdk
    @left-curve/encoding --> @left-curve/sdk
    @left-curve/config --> @left-curve/react
    @left-curve/types --> @left-curve/react
    @left-curve/connectkit --> @left-curve/react
    @left-curve/crypto --> @left-curve/react
    @left-curve/encoding --> @left-curve/react
    @left-curve/utils --> @left-curve/react
    @left-curve/config --> @left-curve/encoding
    @left-curve/types --> @left-curve/encoding
    @left-curve/utils --> @left-curve/encoding
    @left-curve/config --> @left-curve/crypto
    @left-curve/types --> @left-curve/crypto
    @left-curve/encoding --> @left-curve/crypto
    @left-curve/config --> @left-curve/connectkit
    @left-curve/crypto --> @left-curve/connectkit
    @left-curve/encoding --> @left-curve/connectkit
    @left-curve/sdk --> @left-curve/connectkit
    @left-curve/types --> @left-curve/connectkit
    @left-curve/utils --> @left-curve/connectkit
```

## Supported JS environments

Packages in the workspace are compiled to JavaScript ES2021, targeting the latest ECMAScript standard, and support both ESM and CJS module formats.

1. Node.js 18+
2. Modern browsers (Chromium/Firefox/Safari)
3. Browser extensions (Chromium/Firefox)

## Acknowledgement

This project draws inspiration from and follows some of the architectural design principles of [Viem], while utilizing foundational code from [Wagmi]. Several concepts and ideas have been directly adapted from their codebase, significantly influencing this project.

Additionally, we would like to acknowledge [CosmJS] for providing essential code and tools that contributed to key aspects of this project.

We are grateful to both the [Wevm] and [Confio] team for their open-source contributions and the valuable support they offer to the community.

## License

TBD

[Grug]: https://github.com/left-curve/grug
[Wevm]: https://wevm.dev/
[Wagmi]: https://github.com/wevm/wagmi
[Viem]: https://github.com/wevm/viem
[CosmJS]: https://github.com/cosmos/cosmjs
[Confio]: https://confio.gmbh/
