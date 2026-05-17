# @left-curve/sdk

## 1.0.0

### Major Changes

- [#2044](https://github.com/left-curve/left-curve/pull/2044) [`07bfd59`](https://github.com/left-curve/left-curve/commit/07bfd59602d39c6efbf92e0a47370b6597c80fcf) Thanks [@j0nl1](https://github.com/j0nl1)! - Split `@left-curve/sdk` into independent packages for lighter dependency trees.

  ### New packages

  - `@left-curve/types` — TypeScript type definitions
  - `@left-curve/encoding` — Hex, base64, binary, JSON encoding utilities
  - `@left-curve/utils` — Decimal math, formatting, assertions, general utilities
  - `@left-curve/crypto` — Hash functions, key pairs, WebAuthn primitives

  ### Breaking changes

  - **Removed subpath exports**: `@left-curve/sdk/types`, `@left-curve/sdk/encoding`, `@left-curve/sdk/utils`, `@left-curve/sdk/crypto` — import from the standalone packages instead
  - **Removed `grugActions`/`GrugActions`**: query actions merged into `AppQueryActions`, available via `@left-curve/sdk/actions`
  - **Removed `getAction` indirection**: all actions call their implementation directly
  - **Removed cometbft actions**: `queryAbci`, top-level `queryStatus` removed
  - **SDK root export slimmed down**: only re-exports commonly used types, `formatUnits`, `parseUnits`, `Secp256k1`, and all perps types. Import specific packages for full access.

  ### Migration

  ```typescript
  // Before
  import type { Coin } from "@left-curve/sdk/types";
  import { encodeHex } from "@left-curve/sdk/encoding";
  import { Decimal } from "@left-curve/sdk/utils";
  import { grugActions } from "@left-curve/sdk";

  // After
  import type { Coin } from "@left-curve/types";
  import { encodeHex } from "@left-curve/encoding";
  import { Decimal } from "@left-curve/utils";
  import { publicActions } from "@left-curve/sdk/actions";
  ```

### Patch Changes

- Updated dependencies [[`07bfd59`](https://github.com/left-curve/left-curve/commit/07bfd59602d39c6efbf92e0a47370b6597c80fcf)]:
  - @left-curve/types@1.0.0
  - @left-curve/encoding@1.0.0
  - @left-curve/utils@1.0.0
  - @left-curve/crypto@1.0.0
