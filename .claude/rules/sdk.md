---
paths:
  - "sdk/typescript/**/*.{ts,tsx}"
---
# SDK Rules (TypeScript Libraries)

## Design Principles

- Pure library code — no React, no UI framework dependencies
- Minimal public API surface: only export what consumers need
- Zero side effects in module initialization

## Client Architecture

Factory functions + extension pattern. No class-based clients.

```typescript
// Factory creates base client
export function createGrugClient<T extends Transport>(
  parameters: GrugClientConfig<T>,
): GrugClient<T> {
  const client = createBaseClient({ ...parameters, type: "grugClient" });
  return client.extend(grugActions);
}

// Extension adds domain actions
const publicClient = client.extend(publicActions);
const signerClient = publicClient.extend(signerActions);
```

Clients expose: `transport`, `request`, `subscribe`, and `extend()`.

## Type System

Heavy use of generics and utility types for type-safe APIs:

```typescript
// Generic constraints with defaults
function createClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = undefined,
>(config: Config<transport, chain>): Client<transport, chain>
```

Key utility types:
- `Result<T, E>` / `Option<T>` — error handling and optionality
- `Prettify<T>` — flatten intersection types
- `OneOf<T>` — discriminated unions
- `RequiredBy<T, K>` — conditional requirements
- `StrictOmit<T, K>` — type-safe omit
- `MaybePromise<T>` — Promise or sync value

Action signatures follow `Parameters` / `ReturnType` naming:

```typescript
type GetBalanceParameters = { address: string; denom: string };
type GetBalanceReturnType = Coin;

async function getBalance<chain, signer>(
  client: Client<Transport, chain, signer>,
  parameters: GetBalanceParameters,
): Promise<GetBalanceReturnType>
```

## Transport Schema System

Type-safe request/response pairs:

```typescript
type TransportSchema = readonly {
  Method: string;
  Parameters?: unknown;
  ReturnType?: unknown;
}[];
```

## Module Organization

By domain: `/actions/app`, `/actions/dex`, `/actions/perps`, etc.
Each domain has:
- `/mutations` — state-changing operations
- `/queries` — read-only operations
- `domainActions.ts` — action builder combining both

```typescript
// Domain actions builder
export function dexActions<chain, signer>(
  client: Client<Transport, chain, signer>,
) {
  return {
    swap: (params: SwapParameters) => swap(client, params),
    getPool: (params: GetPoolParameters) => getPool(client, params),
  };
}
```

## Error Handling

Hierarchical custom errors extending `BaseError`:

```typescript
export class HttpRequestError extends BaseError {
  override name = "HttpRequestError";
  constructor(args: { cause?: Error; url: string }) {
    super("HTTP request failed.", { cause: args.cause });
  }
}
```

Structured metadata: `shortMessage`, `details`, `metaMessages`.

## Configuration

Flat options objects passed to factories (no builder pattern):

```typescript
type HttpTransportConfig = {
  fetchOptions?: HttpClientOptions["fetchOptions"];
  batch?: boolean | JsonRpcBatchOptions;
  timeout?: number;
};

export function http(url?: string, config: HttpTransportConfig = {}) { ... }
```

## Build & Exports

- `tsup` for bundling
- `.js` extensions in all imports
- Barrel files with explicit type exports
- `TypeDoc` for documentation generation

## Dependencies

- `big.js` for arbitrary precision math (UI wraps this as `Decimal`)
- `@noble/curves` / `@noble/hashes` for cryptography
- `eventemitter3` for event system
- `graphql` / `graphql-ws` for transport

## Testing

- Vitest with `*.spec.ts` convention
- Tests colocated with source files
- Edge cases and convenience function coverage
