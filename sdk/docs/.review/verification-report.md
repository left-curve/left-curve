# SDK ↔ Protocol Book Verification Report

This pass cross-checks every SDK action page that wraps a protocol concept against
the protocol book under `book/`. The book is canonical; SDK pages were aligned
to it where they drifted.

## Pages checked

### Perps actions — order matching (`book/perps/2-order-matching.md`)

TypeScript (`sdk/docs/pages/typescript/actions/perps/`):
- `submitPerpsOrder.mdx`
- `cancelPerpsOrder.mdx`
- `submitConditionalOrder.mdx`
- `submitConditionalOrders.mdx`
- `cancelConditionalOrder.mdx`
- `getPerpsOrdersByUser.mdx`
- `getPerpsLiquidityDepth.mdx`
- `getPerpsPairState.mdx`
- `getPerpsPairParam.mdx`
- `getPerpsPairParams.mdx`
- `getPerpsParam.mdx`
- `getPerpsState.mdx`

Python (`sdk/docs/pages/python/api/methods/exchange/` and `…/info/`):
- `submit_order.mdx`, `submit_limit_order.mdx`, `submit_market_order.mdx`
- `cancel_order.mdx`
- `submit_conditional_order.mdx`, `cancel_conditional_order.mdx`
- `batch_update_orders.mdx`
- `order.mdx`, `orders_by_user.mdx`
- `liquidity_depth.mdx`
- `pair_state.mdx`, `pair_states.mdx`, `pair_param.mdx`, `pair_params.mdx`
- `perps_param.mdx`, `perps_state.mdx`

### Perps actions — margin (`book/perps/1-margin.md`)

TypeScript:
- `depositMargin.mdx`
- `withdrawMargin.mdx`
- `getPerpsUserState.mdx`
- `getPerpsUserStateExtended.mdx`

Python:
- `deposit_margin.mdx`
- `withdraw_margin.mdx`
- `user_state.mdx`, `user_state_extended.mdx`

### Perps actions — funding (`book/perps/3-funding.md`)

- `getPerpsPairState.mdx` (TS), `pair_state.mdx` (Py) — `funding_per_unit`, `funding_rate`

### Perps actions — liquidation / ADL (`book/perps/4-liquidation-and-adl.md`)

- `liquidate.mdx` (Py only — TS has no first-class action)
- `user_state_extended.mdx` (Py), `getPerpsUserStateExtended.mdx` (TS) — liquidation price field
- `queryPerpsEvents.mdx` (TS), `perps_events.mdx` (Py) — `liquidated` / `deleveraged` event types

### Perps actions — vault (`book/perps/5-vault.md`)

TypeScript:
- `vaultAddLiquidity.mdx`
- `vaultRemoveLiquidity.mdx`
- `getPerpsVaultState.mdx`
- `getVaultSnapshots.mdx`

Python:
- `add_liquidity.mdx`
- `remove_liquidity.mdx`

### Perps actions — referral (`book/perps/6-referral.md`)

- `setReferral.mdx` (TS), `set_referral.mdx` (Py)
- `setFeeShareRatio.mdx` (TS)
- `getFeeRateOverride.mdx` (TS)

### Account-factory actions (`book/overview/3-dango-contracts.md` §3)

- `registerUser.mdx`, `registerAccount.mdx`, `createSession.mdx`, `updateKey.mdx`,
  `updateUsername.mdx`, `getUser.mdx`, `getAccountInfo.mdx`

### Rust SDK

The Rust SDK only ships the low-level HTTP/WS client, signer, and keystore
primitives — there are no perps-specific or account-factory-specific action
pages to verify or enrich. The Rust pages are aligned with their actual
domain (transports, signing, subscriptions) and require no protocol-book
cross-check.

## Contradictions found and fixed

### 1. `withdrawMargin` (TypeScript) — wrong unit on `amount`

**Page:** `sdk/docs/pages/typescript/actions/perps/withdrawMargin.mdx`

**Contradiction:** The page described `amount` as "Amount in base units."

**Book reference:** `book/perps/1-margin.md` §3 ("Trader Withdraw"): _"The user
specifies how much USD margin to withdraw … Converts USD to settlement
currency tokens at the fixed \$1 rate (floor-rounded to base units)."_

The on-chain message body is `{ trade: { withdraw: { amount } } }` with no
attached funds — confirming `amount` is a USD value, not a base-unit token
quantity. The Python `withdraw_margin` page already correctly documents this
("Amount is in **USD** (6-decimal `UsdValue`)").

**Fix:** Updated `amount` parameter description to specify USD value (6-decimal
`UsdValue` wire form) and added a `Notes` clause explaining the asymmetry with
`depositMargin` (which takes base units of `bridge/usdc`).

### 2. `vaultAddLiquidity` (TypeScript) — wrong unit on `amount`

**Page:** `sdk/docs/pages/typescript/actions/perps/vaultAddLiquidity.mdx`

**Contradiction:** The page described `amount` as "Deposit amount in base
units."

**Book reference:** `book/perps/5-vault.md` §2 ("Liquidity provision /
Share minting"): _"The LP specifies a USD margin amount `depositMargin` to
transfer from their trading margin to the vault."_

The on-chain message body is `{ vault: { addLiquidity: { amount } } }` with no
attached funds — the amount is debited from the caller's existing trading
margin in USD. The Python `add_liquidity` page already correctly documents this.

**Fix:** Updated `amount` parameter description to specify USD value and added
a `Notes` clause explaining the deposit comes from the caller's existing
trading margin (no token transfer at the wallet boundary).

## Pages that were already aligned

All other action pages were consistent with the book:

- TS `depositMargin` correctly says `bridge/usdc` base units (matches book §2
  margin: settlement currency at fixed \$1).
- TS `submitPerpsOrder` correctly describes positive size = long, negative =
  short (matches book §1 order matching).
- TS `vaultRemoveLiquidity` correctly explains the cooldown unlock (matches
  book §3 vault).
- TS `setFeeShareRatio` correctly notes the 50% cap implicitly via
  truncation; the cap is enforced on-chain per book §3a referral.
- Python `submit_order`, `submit_limit_order`, `submit_market_order`,
  `submit_conditional_order` correctly model TIF semantics, conditional
  reduce-only, slippage caps.
- Python `liquidate` correctly notes the permissionless caller property
  (matches book §1 liquidation: any user can liquidate any other user with
  `equity < MM`).
- Python `withdraw_margin`, `add_liquidity` correctly document USD amounts.
- Pair-state, user-state, and event-stream pages reflect the book's field set
  (long/short OI, funding accumulator, equity, MM, etc.) without contradicting
  it.

## Pass 2 (enrichment) summary

After Pass 1, every action page that wraps a real protocol mechanic received
either a 1-sentence concept lead-in above `## Signature` and/or a Notes
clarification grounded in the book. Pages that are pure data plumbing
(`getCodeHash`, `getNextAccountIndex`, `getAccountSeenNonces`,
`getAllAccountInfo`, `forgotUsername`, pagination cursors, indexer
subscriptions) were left untouched.

Top three enrichment themes:

1. **Units / accounting boundaries** — clarifying when an amount is base
   units of a settlement token vs. an internal USD value
   (`depositMargin` vs `withdrawMargin`, `vaultAddLiquidity`,
   `vaultRemoveLiquidity`, `liquidity_depth`).
2. **Pre-trade rejection causes** — the per-pair tick size, price-band,
   slippage cap, OI cap, and pre-match margin checks that can revert order
   submission (`submitPerpsOrder`, `submit_limit_order`,
   `submit_market_order`).
3. **Funding / liquidation lifecycle** — explaining that funding settles
   lazily on every position touch, that liquidation fees go to the insurance
   fund (not the vault), and that ADL fills carry no fee
   (`getPerpsPairState`, `getPerpsUserStateExtended`, `liquidate`,
   `queryPerpsEvents`).
