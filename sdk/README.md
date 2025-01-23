## Grug SDK

You can find more info in our [documentation](https://grug-sdk.pages.dev/).

## Packages

#### `/packages`

| Package                  | Description                                                                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`config`](./config)     | Common configurations for tsconfig, tsup, biome, typedoc                                                                                                    |
| [`dango`](./dango/)      | The SDK package provides a set of tools and utilities to interact with [Dango] chain                                                                        |
| [`crypto`](./crypto)     | It includes various cryptographic functions and utilities for encryption, decryption, hashing, and more.                                                    |
| [`encoding`](./encoding) | Encoding helpers that are used across packages and assist in encoding and decoding data in various formats                                                  |
| [`grug-sdk`](./grug)     | The SDK package provides a set of tools and utilities to interact with [Grug] execution environment                                                         |
| [`types`](./types)       | It contains various type definition used across the codebase. These types help ensure type safety and improve code readbility.                              |
| [`utils`](./utils)       | Its a collection of utility functions that are used across the project. These utilities are designed to simplify common tasks and improve code reusability. |

## Module Graph

```mermaid
%%{
  init: {
  'theme': 'base',
  'themeVariables': {"primaryTextColor":"#fff","primaryColor":"#5a4f7c","lineColor":"#f5a623" }
  }
}%%
stateDiagram-v2
    state "@left-curve/config" as left_curve_config
    state "@left-curve/types" as left_curve_types
    state "@left-curve/utils" as left_curve_utils
    state "@left-curve/grug" as left_curve_sdk
    state "@left-curve/dango" as left_curve_dango

    left_curve_config --> left_curve_types
    left_curve_config --> left_curve_utils
    left_curve_types --> left_curve_utils
    left_curve_config --> left_curve_sdk
    left_curve_types --> left_curve_sdk
    left_curve_utils --> left_curve_sdk
    left_curve_config --> left_curve_dango
    left_curve_sdk --> left_curve_dango
    left_curve_types --> left_curve_dango
    left_curve_utils --> left_curve_dango
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

[Grug]: https://grug.build/
[Dango]: ../dango
[Wevm]: https://wevm.dev/
[Wagmi]: https://github.com/wevm/wagmi
[Viem]: https://github.com/wevm/viem
[CosmJS]: https://github.com/cosmos/cosmjs
[Confio]: https://confio.gmbh/
