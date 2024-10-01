## Interface

You can find more info in our [documentation](https://ui-doc.pages.dev/).

## Packages

#### `/apps`

| App         | Description |
| ----------- | ----------- |
| [PortalApp] | -           |

#### `/packages`

| Package                                  | Description                                                                                                                                                 |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`config`](./packages/config)            | Common configurations for tsconfig, tsup, biome, typedoc and tailwind                                                                                       |
| [`connect-kit`](./packages/connect-kit/) | It allows connection to multiple blockchains and wallets, manages accounts, and enables interaction with smart contracts.                                   |
| [`crypto`](./packages/crypto)            | It includes various cryptographic functions and utilities for encryption, decryption, hashing, and more.                                                    |
| [`ui`](./packages/ui)                    | React components, hooks, providers and others that help to build [PortalApp]. It includes a [Storybook] server.                                             |
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
    @leftcurve/config --> @leftcurve/types
    @leftcurve/config --> @leftcurve/utils
    @leftcurve/types --> @leftcurve/utils
    @leftcurve/config --> @leftcurve/sdk
    @leftcurve/types --> @leftcurve/sdk
    @leftcurve/utils --> @leftcurve/sdk
    @leftcurve/crypto --> @leftcurve/sdk
    @leftcurve/encoding --> @leftcurve/sdk
    @leftcurve/config --> @leftcurve/react
    @leftcurve/types --> @leftcurve/react
    @leftcurve/connectkit --> @leftcurve/react
    @leftcurve/crypto --> @leftcurve/react
    @leftcurve/encoding --> @leftcurve/react
    @leftcurve/utils --> @leftcurve/react
    @leftcurve/config --> @leftcurve/encoding
    @leftcurve/types --> @leftcurve/encoding
    @leftcurve/utils --> @leftcurve/encoding
    @leftcurve/config --> @leftcurve/ui
    @leftcurve/react --> @leftcurve/ui
    @leftcurve/utils --> @leftcurve/ui
    @leftcurve/sdk --> @leftcurve/ui
    @leftcurve/types --> @leftcurve/ui
    @leftcurve/config --> @leftcurve/crypto
    @leftcurve/types --> @leftcurve/crypto
    @leftcurve/encoding --> @leftcurve/crypto
    @leftcurve/config --> @leftcurve/connectkit
    @leftcurve/crypto --> @leftcurve/connectkit
    @leftcurve/encoding --> @leftcurve/connectkit
    @leftcurve/sdk --> @leftcurve/connectkit
    @leftcurve/types --> @leftcurve/connectkit
    @leftcurve/utils --> @leftcurve/connectkit
```


## Supported JS environments
Packages in the workspace are compiled to JavaScript ES2021, targeting the latest ECMAScript standard, and support both ESM and CJS module formats.

1. Node.js 18+
2. Modern browsers (Chromium/Firefox/Safari)
3. Browser extensions (Chromium/Firefox)

## Development

See [Hacking.md]

## Acknowledgement

This project draws inspiration from and follows some of the architectural design principles of [Viem], while utilizing foundational code from [Wagmi]. Several concepts and ideas have been directly adapted from their codebase, significantly influencing this project.

Additionally, we would like to acknowledge [CosmJS] for providing essential code and tools that contributed to key aspects of this project.

We are grateful to both the [Wevm] and [Confio] team for their open-source contributions and the valuable support they offer to the community.

## License

TBD

[Grug]: https://github.com/left-curve/grug
[Wevm]: https://wevm.dev/
[Wagmi]: https://github.com/wevm/wagmi
[Storybook]: https://storybook.js.org/
[PortalApp]: ./apps/portal
[Hacking.md]: ./HACKING.md
[Viem]: https://github.com/wevm/viem
[CosmJS]: https://github.com/cosmos/cosmjs
[Confio]: https://confio.gmbh/
