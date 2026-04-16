# Perps trade form — max-size math

This note explains the "Available to trade" and slider-max calculations
used by `PerpsTradeMenu` (in `TradeMenu.tsx`, via the `usePerpsMaxSize`
hook in `ui/store/src/hooks/usePerpsMaxSize.ts`). The UX mirrors
Hyperliquid: both values are **side-dependent** and react to the user's
existing position, selected leverage, and the reduce-only flag.

## Why the maths is non-trivial

A closing order behaves very differently from an opening order on-chain:

- For **non-reduce-only** orders, the chain's pre-match check is

  ```plain
  equity ≥ |current_position + order_size|·oracle·IMR
           + |order_size|·oracle·fee`.
  ```

  When the order opposes the existing position, the projected IM term
  _shrinks_ (partial close) or goes to zero (full close), so large
  sells against a long pass even when `availableMargin` is at zero.
- For **reduce-only** orders, the chain skips the margin check entirely
  and forces the opening portion to zero — the only requirement is
  that the fillable closing size is positive.

Naïvely using `availableMargin / (1/L + fee) / price` for every order
would lock users out of closing a position once margin is tied up,
which is the bug the fix resolves. The formulas below reproduce the
chain's actual acceptance criterion under a Hyperliquid-style
presentation.

## Variables

| Symbol        | Meaning                                                         |
| ------------- | --------------------------------------------------------------- |
| `equity`      | Total user equity (from the extended perps user-state).         |
| `pos`         | Signed base-unit size of the position in the traded pair.       |
| `mark`        | Mark price (pair stats first, oracle fallback).                 |
| `L`           | User-selected leverage in the UI (not the pair's max).          |
| `fee`         | Taker fee rate as a decimal (e.g. `0.00038` for 0.038%).        |
| `IM_pos_at_L` | `abs(pos) · mark / L` — the position's IM at selected leverage. |

An order is **opposing** iff `sign(pos) ≠ sign(orderDirection)` and
`pos ≠ 0`. Otherwise it's **same-side** (or there's no position).

## Formulas

```plain
availToTrade =
    equity                           when |pos| = 0
    equity − IM_pos_at_L             when same-side (buying more long, selling more short)
    equity + IM_pos_at_L             when opposing (selling a long, buying a short)

# Non-reduce-only
max_notional = availToTrade / (1/L + fee)
max_base     = max_notional / mark

# Reduce-only, opposing
max_notional = |pos| · mark
max_base     = |pos|

# Reduce-only, same-side or no position
max = 0                              # slider + submit disabled
```

`availToTrade` is clamped at zero before dividing. Reduce-only skips
leverage and fee because the chain doesn't check margin for reduce-only
orders — the only sensible cap is the position itself.

## Why the formula matches the chain

Solving the chain's pre-match check
`equity ≥ |pos + X_signed|·mark·IMR + |X|·mark·fee` for the largest
`X` with `IMR = 1/L` yields:

```plain
max_notional = (equity + |pos|·mark·IMR) / (IMR + fee)     # opposing
             = availToTrade / (1/L + fee)
```

so the Hyperliquid framing and the on-chain constraint are the same
expression. The UI uses the user-selected `L` (not the pair's fixed
IMR), which makes the slider conservative relative to what the chain
would accept — a safety margin, not a rejection risk.

## Worked examples (24 rows)

Inputs used across all three sections:

```plain
equity = $500
mark   = $75,000
fee    = 0.00038 (0.038%)
```

Each section treats equity as $500 (independent starting points, not a
continuous sequence).

### A. No position (rows 1–8)

`availToTrade = equity = $500` regardless of side; `IM_pos_at_L = 0`.

| #   | Leverage | Action | Reduce-only | Avail to trade | Slider 100%   | What the order does           |
| --- | -------- | ------ | ----------- | -------------- | ------------- | ----------------------------- |
| 1   | 1×       | buy    | off         | $500.00        | $499.81       | opens long                    |
| 2   | 1×       | buy    | **on**      | $500.00        | $0 (disabled) | RO requires opposing position |
| 3   | 1×       | sell   | off         | $500.00        | $499.81       | opens short                   |
| 4   | 1×       | sell   | **on**      | $500.00        | $0 (disabled) | RO requires opposing position |
| 5   | 2×       | buy    | off         | $500.00        | $999.24       | opens long                    |
| 6   | 2×       | buy    | **on**      | $500.00        | $0 (disabled) | RO requires opposing position |
| 7   | 2×       | sell   | off         | $500.00        | $999.24       | opens short                   |
| 8   | 2×       | sell   | **on**      | $500.00        | $0 (disabled) | RO requires opposing position |

### B. $500 BTC long, i.e. `pos = +0.006667 BTC` (rows 9–16)

`IM_pos_at_1x = $500`, `IM_pos_at_2x = $250`. Buy is same-side, sell
is opposing.

| #   | Leverage | Action | Reduce-only | Avail to trade | Slider 100%   | What the order does                  |
| --- | -------- | ------ | ----------- | -------------- | ------------- | ------------------------------------ |
| 9   | 1×       | buy    | off         | $0.00          | $0 (disabled) | position absorbs all margin at 1×    |
| 10  | 1×       | buy    | **on**      | $0.00          | $0 (disabled) | same-side; RO requires opposing      |
| 11  | 1×       | sell   | off         | $1,000.00      | $999.62       | closes $500, flips to ~$499.62 short |
| 12  | 1×       | sell   | **on**      | $1,000.00      | $500.00       | pure close (capped at position)      |
| 13  | 2×       | buy    | off         | $250.00        | $499.62       | adds to long                         |
| 14  | 2×       | buy    | **on**      | $250.00        | $0 (disabled) | same-side; RO requires opposing      |
| 15  | 2×       | sell   | off         | $750.00        | $1,498.86     | closes $500, flips to ~$998.86 short |
| 16  | 2×       | sell   | **on**      | $750.00        | $500.00       | pure close (capped at position)      |

### C. $250 BTC short, i.e. `pos = −0.003333 BTC` (rows 17–24)

`IM_pos_at_1x = $250`, `IM_pos_at_2x = $125`. Buy is opposing, sell
is same-side.

| #   | Leverage | Action | Reduce-only | Avail to trade | Slider 100%   | What the order does                 |
| --- | -------- | ------ | ----------- | -------------- | ------------- | ----------------------------------- |
| 17  | 1×       | buy    | off         | $750.00        | $749.72       | closes $250, flips to ~$499.72 long |
| 18  | 1×       | buy    | **on**      | $750.00        | $250.00       | pure close (capped at position)     |
| 19  | 1×       | sell   | off         | $250.00        | $249.91       | adds to short                       |
| 20  | 1×       | sell   | **on**      | $250.00        | $0 (disabled) | same-side; RO requires opposing     |
| 21  | 2×       | buy    | off         | $625.00        | $1,249.05     | closes $250, flips to ~$999.05 long |
| 22  | 2×       | buy    | **on**      | $625.00        | $250.00       | pure close (capped at position)     |
| 23  | 2×       | sell   | off         | $375.00        | $749.43       | adds to short                       |
| 24  | 2×       | sell   | **on**      | $375.00        | $0 (disabled) | same-side; RO requires opposing     |

## Code pointers

- Hook: `ui/store/src/hooks/usePerpsMaxSize.ts` — the entire formula
  lives here; takes `equity`, `currentPositionSize`, `action`,
  `leverage`, `currentPrice`, `takerFeeRate`, `reduceOnly`,
  `isBaseSize` and returns `{ availToTrade, maxSize }`.
- Consumer: `TradeMenu.tsx` `PerpsTradeMenu` — reads the hook once,
  feeds `availToTrade` into the "Available to trade" row and
  `maxSize` (aliased as `maxSizeAmount`) into the slider, size input
  validator, clamp effect, and submit-button disable condition.
- The disable condition `reduceOnly && maxSizeAmount === 0` covers
  rows 2/4/6/8/10/14/20/24 (slider + submit greyed out, helper text
  visible). Non-RO rows with `maxSize === 0` (row 9 — full margin
  committed at 1×) also disable the slider and leave the submit
  button inactive because the size auto-clamps to zero.

## Chain-side reference

- `dango/perps/src/core/decompose.rs` — decomposition into closing and
  opening portions.
- `dango/perps/src/core/margin.rs` (`check_margin`) — the pre-match
  margin check the UI formula mirrors.
- `dango/perps/src/trade/submit_order.rs` — reduce-only short-circuit
  (zeros the opening portion, skips the margin check).
- Spec: `book/perps/2-order-matching.md` §2 (decomposition), §5
  (pre-match margin check).
