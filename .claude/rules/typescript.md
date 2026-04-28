---
paths:
  - "**/*.{ts,tsx}"
---
# TypeScript Guidelines (Shared)

## Naming

| Type | Convention | Example |
|------|-----------|---------|
| Source files | camelCase | `baseClient.ts`, `grugClient.ts` |
| React components | PascalCase | `Button.tsx`, `AccountMenu.tsx` |
| Directories | lowercase | `clients/`, `crypto/`, `types/` |
| Test files | `.spec.ts` suffix | `binary.spec.ts` |
| Story files | `.stories.tsx` suffix | `Button.stories.tsx` |

## Exports & Imports

- Named exports only, no default exports
- Barrel files (`index.ts`) with explicit type re-exports
- `.js` extensions in all imports
- Wildcard re-exports only in barrel files

```typescript
// Barrel file pattern
export { createBaseClient } from "./baseClient.js";
export { type GrugClient, createGrugClient } from "./grugClient.js";
export * from "./webauthn/index.js";
```

Import order: external -> internal -> types

```typescript
// External
import { secp256k1 } from "@noble/curves/secp256k1";
// Internal
import { encodeHex } from "../../encoding/hex.js";
// Types
import type { Hex } from "../../types/index.js";
```

## Types

- `interface` for contracts and structural patterns (extendable)
- `type` for unions, aliases, complex types
- Never use `any` — use `unknown` and narrow

```typescript
// Interfaces for contracts
interface HashFunction {
  readonly blockSize: number;
  update(data: Uint8Array): HashFunction;
  digest(): Uint8Array;
}

// Types for everything else
type Result<T, E = Error> = Success<T> | Failure<E>;
type Option<T> = T | undefined;
```

## Code Style

- `const` over `let`, never `var`
- Arrow functions as standard
- Immutable operations: `reduce`/`filter`/`map` over mutable loops
- Early returns over nested conditionals
- Destructuring with defaults
- Nullish coalescing (`??`) and optional chaining (`?.`)

```typescript
// BAD — mutable accumulation
let total = 0;
for (const item of items) { total += item.price * item.qty; }

// GOOD — immutable, Decimal
const total = items.reduce((sum, i) => sum.add(i.price.mul(i.qty)), Decimal(0));
```

### IIFE for Scoped Initialization

```typescript
const apiUrl = (() => {
  if (process.env.NODE_ENV === "production") return "https://api.prod.com";
  if (process.env.NODE_ENV === "staging") return "https://api.staging.com";
  return "http://localhost:3000";
})();
```

### Immutable Operations

```typescript
// Spread for copies
const newArray = [...items, newItem];
const newObject = { ...config, timeout: 3000 };

// Non-mutating array methods
const sorted = [...items].sort((a, b) => a - b);
const updated = items.map((item) => item.id === id ? { ...item, active: true } : item);

// BAD — mutations
items.push(newItem);
items.sort();
config.timeout = 3000;
```

## Arithmetic

Always use `Decimal` for arithmetic, never `Number`. Decimal wraps big.js.
Floating point bugs are silent and catastrophic in financial code.

## Error Handling

Custom error hierarchy extending `BaseError` with structured metadata:

```typescript
export class HttpRequestError extends BaseError {
  override name = "HttpRequestError";
  constructor(args: { cause?: Error; url: string }) {
    super("HTTP request failed.", { cause: args.cause });
  }
}

// Usage with instanceof
try { /* ... */ } catch (err) {
  if (err instanceof HttpRequestError) throw err;
  throw new HttpRequestError({ cause: err as Error, url });
}
```

## Classes

- Private fields with `#` syntax
- Static factory methods for construction

```typescript
export class Secp256k1 implements KeyPair {
  #privateKey: Uint8Array;

  static fromMnemonic(mnemonic: string): Secp256k1 {
    const seed = mnemonicToSeedSync(mnemonic);
    return new Secp256k1(deriveKey(seed));
  }

  constructor(privateKey: Uint8Array) {
    this.#privateKey = privateKey;
  }
}
```

## Patterns

- Factory functions for object creation
- Async/await exclusively (no raw `.then()` chains)
- Never duplicate code >3 lines — extract immediately
