# @left-curve/utils

General-purpose utilities for the [Dango](https://dango.exchange) ecosystem.

## Installation

```bash
npm install @left-curve/utils
```

## Usage

```typescript
import { Decimal, formatUnits, parseUnits, truncateAddress } from "@left-curve/utils";

const amount = Decimal("1000.50");
const formatted = formatUnits("1000000", 6); // "1.000000"
const parsed = parseUnits("1.5", 6); // "1500000"
const short = truncateAddress("0x1234...abcd");
```

## API

### Decimal

Arbitrary-precision decimal arithmetic (wraps `big.js`). Use for all financial math — never use `Number`.

```typescript
import { Decimal } from "@left-curve/utils";

const a = Decimal("100.5");
const b = a.mul(Decimal("2"));
```

### Formatting

- `formatNumber(value, options)` - locale-aware number formatting
- `formatUnits(value, decimals)` / `parseUnits(value, decimals)` - unit conversion
- `truncateAddress(address)` - shorten addresses for display
- `truncateDec(value, digits)` - truncate decimal places

### Strings

- `camelToSnake(str)` / `snakeToCamel(str)` - case conversion
- `capitalize(str)` - capitalize first letter

### Assertions

- `assertString(value)`, `assertNumber(value)`, `assertBoolean(value)`, etc.
- `assertDeepEqual(a, b)` - deep equality check
- `assertNotEmpty(value)` - non-empty check

### Async

- `withRetry(fn, options)` - retry with backoff
- `withTimeout(fn, timeout)` - timeout wrapper
- `wait(ms)` - promise-based delay
- `withResolvers()` - deferred promise

### Objects

- `recursiveTransform(obj, fn)` - deep transform object values
- `sortObject(obj)` - sort object keys
- `invertObject(obj)` - swap keys and values

### DEX

- `calculateTradeSize(params)`, `calculateFees(params)`, `calculatePrice(params)`
- `adjustPrice(price, tickSize)` - snap price to tick
- `formatOrderId(id)` - format order ID for display

### Vault

- `sharesToUsd(shares, params)` / `usdToShares(usd, params)` - vault conversions
- `computeVaultApy(snapshots)` - compute vault APY from snapshots

### Misc

- `uid()` - generate unique ID
- `debounce(fn, delay)` - debounce function calls
- `tryCatch(fn)` - Result type wrapper
- `createSubscription(options)` - subscription helper
- `batchPoller(fn, interval)` - batch polling
- `getNavigatorOS()`, `isMobileOrTable()` - browser detection

## License

TBD
