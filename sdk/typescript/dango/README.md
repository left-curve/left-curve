# @left-curve/sdk

The SDK package for interacting with the [Dango](https://dango.exchange) execution environment.

## Installation

```bash
npm install @left-curve/sdk
```

For lighter installs, use the individual packages:

```bash
npm install @left-curve/types     # type definitions only
npm install @left-curve/encoding  # encoding utilities only
npm install @left-curve/utils     # general utilities only
npm install @left-curve/crypto    # cryptographic primitives only
```

## Usage

```typescript
import { createPublicClient, createSignerClient, testnet } from "@left-curve/sdk";
import { getBalance } from "@left-curve/sdk/actions";

const client = createPublicClient({ chain: testnet });
const balance = await getBalance(client, { address, denom: "dango" });
```

## Package Structure

The SDK is split into independent packages for lighter dependency trees:

| Package | Description |
|---------|-------------|
| [`@left-curve/types`](../types) | TypeScript type definitions |
| [`@left-curve/encoding`](../encoding) | Hex, base64, binary, JSON encoding |
| [`@left-curve/utils`](../utils) | Decimal math, formatting, assertions |
| [`@left-curve/crypto`](../crypto) | Hash functions, key pairs, WebAuthn |
| `@left-curve/sdk` | Clients, actions, chains, transports |

The root `@left-curve/sdk` import re-exports common items from all sub-packages for convenience.

## Exports

```typescript
// Root - clients, chains, and common re-exports from all sub-packages
import { createPublicClient, testnet, Decimal, sha256 } from "@left-curve/sdk";

// Actions - query and mutation actions
import { getBalance, swap, transfer } from "@left-curve/sdk/actions";
```

## License

TBD
