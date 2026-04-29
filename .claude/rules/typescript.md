---
paths:
  - "**/*.{ts,tsx}"
---

# TypeScript Guidelines (Shared)

## Strict Mode

TypeScript strict mode is mandatory. All `tsconfig.json` files must have `"strict": true`.
Never suppress errors with `@ts-ignore` — use `@ts-expect-error` with an explanation when
genuinely unavoidable (e.g., third-party type bugs).

## Naming

| Type | Convention | Example |
|------|-----------|---------|
| Source files | camelCase | `baseClient.ts`, `grugClient.ts` |
| React components | PascalCase | `Button.tsx`, `AccountMenu.tsx` |
| Directories | lowercase | `clients/`, `crypto/`, `types/` |
| Test files | `.spec.ts` suffix | `binary.spec.ts` |
| Story files | `.stories.tsx` suffix | `Button.stories.tsx` |
| Types/Interfaces | PascalCase | `UserAccount`, `HashFunction` |
| Constants | UPPER_SNAKE_CASE | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |
| Generic params | PascalCase, descriptive | `TTransport`, `TChain` (or single letter for simple cases: `T`, `K`, `V`) |

No `I` prefix on interfaces (`User`, not `IUser`).
No `Enum`/`Type` suffixes unless disambiguation is required.

## Exports & Imports

- Named exports only, no default exports
- Barrel files (`index.ts`) with explicit type re-exports
- `.js` extensions in relative imports within ESM packages (required by module resolution)
- Wildcard re-exports only in barrel files
- Never create mega-barrel files that re-export hundreds of items

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

Always use `import type` / `export type` for type-only imports/exports.

## Types

- `interface` for contracts and structural patterns (extendable)
- `type` for unions, aliases, complex types
- Use whichever communicates intent best — the distinction is a heuristic, not a hard boundary
- Never use `any` — use `unknown` and narrow
- Prefer `readonly` properties by default; only omit when mutation is intentional
- Use `as const` for literal configuration objects and constant data
- Use `satisfies` to validate a value matches a type while preserving its narrower literal type

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

// as const — literal types preserved, readonly enforced
const ROUTES = {
  home: "/",
  trade: "/trade",
  portfolio: "/portfolio",
} as const;

// satisfies — validates shape while keeping the literal type
const config = {
  retries: 3,
  timeout: 5000,
} satisfies Partial<ClientConfig>;
```

### Union Types Over Enums

Prefer string literal unions over enums. They tree-shake better, align with TypeScript's
structural type system, and require no runtime representation.

```typescript
// BAD — enum creates runtime object
enum OrderType { Market = "market", Limit = "limit" }

// GOOD — zero runtime cost
type OrderType = "market" | "limit";
```

### Discriminated Unions

Model variant types with a shared discriminant property. This enables exhaustive checking
and type narrowing in switch/if statements.

```typescript
type TradeAction =
  | { type: "swap"; fromDenom: string; toDenom: string; amount: Decimal }
  | { type: "provide_liquidity"; poolId: string; amounts: Decimal[] }
  | { type: "withdraw_liquidity"; poolId: string; shares: Decimal };

function executeTradeAction(action: TradeAction) {
  switch (action.type) {
    case "swap":
      return handleSwap(action.fromDenom, action.toDenom, action.amount);
    case "provide_liquidity":
      return handleProvide(action.poolId, action.amounts);
    case "withdraw_liquidity":
      return handleWithdraw(action.poolId, action.shares);
    default:
      assertNever(action);
  }
}
```

### Exhaustive Checks

Always use `assertNever` in the `default` branch of discriminated union switches.
This produces a compile-time error when a new variant is added but not handled.

```typescript
function assertNever(x: never): never {
  throw new Error(`Unexpected value: ${x}`);
}
```

### Branded Types

Use branded types to prevent confusion between structurally identical primitives
(e.g., `UserId` vs `OrderId`). Zero runtime cost.

In DeFi context, brand base-unit amounts separately from human-readable values
to prevent silent mis-scaling:

```typescript
type Brand<T, B extends string> = T & { readonly __brand: B };
type Address = Brand<string, "Address">;
type Denom = Brand<string, "Denom">;
type TxHash = Brand<string, "TxHash">;

// DeFi-specific: separate base-unit from human-readable
type AtomicAmount = Brand<bigint, "AtomicAmount">;
type HumanAmount = Brand<Decimal, "HumanAmount">;

function createAddress(raw: string): Address {
  if (!isValidAddress(raw)) throw new AddressError(raw);
  return raw as Address;
}
```

### Type Narrowing and Guards

Prefer type predicates (`is`) for reusable narrowing. Use `in` operator or
discriminant checks for inline narrowing.

```typescript
function isCoin(value: unknown): value is Coin {
  return (
    typeof value === "object" &&
    value !== null &&
    "denom" in value &&
    "amount" in value
  );
}

// Inline narrowing with discriminant
if (result.type === "success") {
  // result is narrowed to Success<T>
}
```

### Template Literal Types

Use template literal types for string patterns when you need compile-time
validation of string formats.

```typescript
type RouteParam = `/${string}`;
type EventName = `on${Capitalize<string>}`;
type QueryKey = readonly [domain: string, ...params: unknown[]];
```

## Generics

- Constrain generics with `extends` to communicate intent
- Provide defaults where a sensible one exists
- Avoid over-generification — only introduce a generic when the function genuinely
  works with multiple types

```typescript
// Constrained with default
function createClient<
  TTransport extends Transport = HttpTransport,
  TChain extends Chain | undefined = undefined,
>(config: ClientConfig<TTransport, TChain>): Client<TTransport, TChain>

// BAD — generic for no reason
function getLength<T extends { length: number }>(arr: T): number { return arr.length; }

// GOOD — just use the constraint directly
function getLength(arr: { length: number }): number { return arr.length; }
```

## Code Style

- `const` over `let`, never `var`
- Arrow functions as standard
- Prefer immutable operations (`map`/`filter`/`reduce`/spread) over mutation.
  Use `for...of` when it is clearer than a `reduce` chain — readability wins over dogma
- Early returns over nested conditionals (guard clause pattern)
- Destructuring with defaults
- Nullish coalescing (`??`) and optional chaining (`?.`)
- `readonly` arrays and tuples when mutation is not needed
- Never use loose equality (`==`) — always strict (`===`)

```typescript
// BAD — mutable accumulation
let total = 0;
for (const item of items) { total += item.price * item.qty; }

// GOOD — immutable, Decimal
const total = items.reduce((sum, i) => sum.add(i.price.mul(i.qty)), Decimal(0));

// ALSO GOOD — for...of when reduce would be convoluted
const errors: string[] = [];
for (const field of fields) {
  const result = validate(field);
  if (!result.ok) errors.push(result.error);
}
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

### Readonly by Default

```typescript
// Prefer readonly for data structures that shouldn't mutate
type PoolState = {
  readonly id: string;
  readonly denoms: readonly string[];
  readonly reserves: readonly Coin[];
};

// Function params — signal "I won't modify your data"
function calculateTotal(items: readonly CartItem[]): Decimal { ... }
```

## Arithmetic

Never use `Number` for financial values — floating point errors are silent and catastrophic.

- **`bigint`** for base-unit / on-chain integer amounts (atomic units, wei-equivalents)
- **`Decimal`** (wraps big.js) for human-readable financial math (prices, rates, display amounts)
- **`number`** is fine for counts, indices, viewport/DOM math, timers, and non-financial values

Always normalize explicitly when converting between base-unit and human-readable:

```typescript
function toHuman(atomic: AtomicAmount, decimals: number): HumanAmount {
  return Decimal(atomic.toString()).div(Decimal(10).pow(decimals)) as HumanAmount;
}
```

### Rounding Policy

Define explicit rounding direction per operation — never rely on default rounding:

- **Fees/deductions**: round UP (user pays at least the fee)
- **Received amounts / "min received"**: round DOWN (user gets at most what's quoted)
- **Display values**: round to token's display precision, truncate (not round) for balances
- **Order submission**: respect tick size and lot size constraints

```typescript
const minReceived = estimatedOutput.mul(Decimal(1).sub(slippage)).round(decimals, Decimal.ROUND_DOWN);
const feeAmount = notional.mul(feeRate).round(decimals, Decimal.ROUND_UP);
```

## Runtime Validation

TypeScript types are erased at runtime. Validate all untrusted data at system boundaries
with Zod (or equivalent runtime schema):

- API responses
- WebSocket payloads
- URL/search params
- Persisted state (localStorage, IndexedDB)
- Wallet/provider responses

```typescript
const CoinSchema = z.object({
  denom: z.string(),
  amount: z.string().transform((v) => Decimal(v)),
});

const balances = CoinSchema.array().parse(rawResponse);
```

## Async Patterns

### No Floating Promises

Every `Promise` must be explicitly handled — never fire-and-forget.
Unhandled rejections are silent bugs.

```typescript
// BAD — promise silently dropped
submitOrder(params);

// GOOD — explicit handling
await submitOrder(params);
submitOrder(params).catch(handleOrderError);
void submitOrder(params); // intentional fire-and-forget, explicitly marked
```

### Cancellation and Cleanup

Use `AbortSignal` for cancellable async work. Always clean up subscriptions
and in-flight requests on unmount or transition.

```typescript
async function fetchWithTimeout(url: string, timeoutMs: number): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}
```

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

Never silently swallow errors. Every `catch` must either:
1. Re-throw (optionally wrapped in a domain error)
2. Return an explicit error value (Result type)
3. Log and recover with a documented fallback

Validate at system boundaries only (user input, API responses, external data).
Trust internal types and framework guarantees — don't add defensive checks
for scenarios the type system already prevents.

### External Data Sanity Checks

After schema validation, financial data needs domain-level sanity checks:

- Reject negative reserves, impossible decimals, absurd price jumps
- Validate token metadata (symbol length, decimal count range)
- Check that quotes/prices are within expected bounds

## Classes

- Private fields with `#` syntax
- Static factory methods for complex construction (multiple init paths, async init)
- Plain constructors when construction is straightforward

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

- Factory functions for object creation (non-class cases)
- Async/await exclusively (no raw `.then()` chains)
- Extract duplicated code when the duplication is repeated and the abstraction is stable.
  Don't extract prematurely after the first occurrence — wait until the pattern is clear

### Conditional Logic

Prefer `switch` with exhaustive `assertNever` for discriminated unions.
Prefer early returns over `else` chains for guard clauses.
Prefer ternary for simple conditional assignment.

```typescript
// Guard clause pattern
function processOrder(order: Order | undefined): Result<Receipt> {
  if (!order) return failure(new OrderNotFoundError());
  if (order.status === "cancelled") return failure(new OrderCancelledError(order.id));
  return success(executeOrder(order));
}
```

### Null Handling

- `undefined` for "not yet loaded" or "absent"
- `null` is acceptable for "intentionally empty / not applicable" when the distinction matters
- Use `Option<T>` (alias for `T | undefined`) for optional values
- Nullish coalescing (`??`) for defaults, optional chaining (`?.`) for access
- Never use loose equality (`==`) for null checks — use strict (`===`) or narrowing
