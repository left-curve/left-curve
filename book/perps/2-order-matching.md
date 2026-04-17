# Order Matching

This chapter describes how orders are submitted, matched, filled, and settled in the on-chain perpetual futures order book.

## 1. Order types

An order can be order:

- **Market** — immediate-or-cancel (IOC). Specifies a `max_slippage` relative to the oracle price. Any unfilled remainder after matching is discarded (unless nothing filled at all, which is an error).
- **Limit** — specifies a `limit_price` and a `time_in_force`:
  - **GTC** (default): any unfilled remainder is stored as a resting order on the book.
  - **IOC**: fills as much as possible, then discards the unfilled remainder. Errors if nothing fills.
  - **Post-only**: the order is to be inserted into the book without entering the matching engine. Reject if it would cross the best price on the opposite side.

Resting orders on the book are stored as:

| Field             | Description                                       |
| ----------------- | ------------------------------------------------- |
| `user`            | Owner address                                     |
| `size`            | Signed quantity (positive = buy, negative = sell) |
| `reduce_only`     | If true, can only close an existing position      |
| `reserved_margin` | Margin locked for this order                      |

The pair ID, order ID, and limit price are part of the storage key.

## 2. Order decomposition

Before matching, every fill is decomposed into a **closing** and an **opening** portion based on the user's current position:

| Order direction | Current position | Closing size                                | Opening size                       |
| --------------- | ---------------- | ------------------------------------------- | ---------------------------------- |
| Buy (+)         | Short (−)        | $\min(\mathtt{size},\; -\mathtt{position})$ | $\mathtt{size} - \mathtt{closing}$ |
| Sell (−)        | Long (+)         | $\max(\mathtt{size},\; -\mathtt{position})$ | $\mathtt{size} - \mathtt{closing}$ |
| Same direction  | Any              | $0$                                         | $\mathtt{size}$                    |

Both closing and opening carry the same sign as the original order size (or are zero). For **reduce-only** orders, the opening portion is forced to zero — if the resulting fillable size is zero, the transaction is rejected.

## 3. Target price

The target price defines the worst acceptable execution price for the taker:

**Market orders (bid/buy):**

$$
\mathtt{targetPrice} = \mathtt{oraclePrice} \times (1 + \mathtt{maxSlippage})
$$

**Market orders (ask/sell):**

$$
\mathtt{targetPrice} = \mathtt{oraclePrice} \times (1 - \mathtt{maxSlippage})
$$

**Limit orders:** $\mathtt{targetPrice} = \mathtt{limitPrice}$ (oracle price is ignored).

The user-supplied $\mathtt{maxSlippage}$ on a market order is bounded by the per-pair cap `max_market_slippage` — see [§3b](#3b-market-order-slippage-cap).

A price constraint is **violated** when:

- Bid: $\mathtt{execPrice} > \mathtt{targetPrice}$
- Ask: $\mathtt{execPrice} < \mathtt{targetPrice}$

## 3a. Price banding for limit orders

Every limit order (GTC, IOC, or post-only) must have a `limit_price` within a per-pair symmetric deviation of the oracle price at submission. Concretely:

$$
\lvert \mathtt{limitPrice} - \mathtt{oraclePrice} \rvert \leq \mathtt{oraclePrice} \times \mathtt{maxLimitPriceDeviation}
$$

where $\mathtt{maxLimitPriceDeviation}$ is a per-pair parameter in $(0, 1)$. Equivalently, the limit price must fall inside

$$
\bigl[ \mathtt{oraclePrice} \times (1 - \mathtt{maxLimitPriceDeviation}),\; \mathtt{oraclePrice} \times (1 + \mathtt{maxLimitPriceDeviation}) \bigr]
$$

An order whose price falls outside this band is rejected at submission, before matching begins. The check is applied identically to GTC, IOC, and post-only limit orders.

## 3b. Market-order slippage cap

Each market order must have a `max_slippage` within a per-pair `max_market_slippage` constraint at submission:

$$
\mathtt{maxSlippage} \leq \mathtt{maxMarketSlippage}
$$

The same cap applies to `max_slippage` on **TP/SL child orders** (attached to a parent submit order or placed as standalone conditional orders), which become market orders when triggered.

**Conditional-order staleness.** It is possible that when a conditional order is submitted, its `max_slippage` falls within the `max_market_slippage` constraint, but when triggered, governance has tightened the constaint such that it is no longer compliant. In this case, the conditional order is canceled with `reason = SlippageCapTightened`.

## 4. Matching engine

The matching engine iterates the opposite side of the book in **price-time priority**:

- A **bid** (buy) walks the asks in ascending price order (cheapest first).
- An **ask** (sell) walks the bids in descending price order (most expensive first). Bids are stored with bitwise-NOT inverted prices so that ascending iteration over storage keys yields descending real prices.

At each resting order the engine checks two **termination conditions**:

1. $\mathtt{remainingSize} = 0$ — the taker is fully filled.
2. The resting order's price violates the taker's price constraint.

If neither condition is met, the fill size is:

$$
\mathtt{takerFillSize} = \begin{cases}
\min(\mathtt{remainingSize},\; \mathtt{makerSize}) & \text{if bid (both positive)} \\
\max(\mathtt{remainingSize},\; \mathtt{makerSize}) & \text{if ask (both negative)}
\end{cases}
$$

$$
\mathtt{makerFillSize} = -\mathtt{takerFillSize}
$$

After each fill the maker order is updated: reserved margin is released proportionally, and if fully filled the order is removed from the book and `open_order_count` is decremented.

## 5. Pre-match margin check

Before matching begins, the taker's margin is verified (skipped for reduce-only orders). The check ensures the user can afford the **worst case** — a 100 % fill:

$$
\mathtt{equity} \geq \mathtt{projectedIM} + \mathtt{projectedFee} + \mathtt{reservedMargin}
$$

where $\mathtt{projectedIM}$ is the initial margin assuming the full order fills (see [Margin §5](1-margin.md#5-initial-margin-im)) and $\mathtt{projectedFee}$ is

$$
|\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{takerFeeRate}
$$

This prevents a taker from submitting orders they cannot collateralise.

## Maker order re-checks

When a maker order with an eligible price is encountered, the matching engine performs two check before executing filling:

### 6a. Self-trade prevention

The exchange uses [`EXPIRE_MAKER`](https://developers.binance.com/docs/derivatives/usds-margined-futures/faq/stp-faq) mode. When the taker encounters their own resting order on the opposite side:

1. The maker (resting) order is **cancelled** (removed from the book).
2. The taker's `open_order_count` and `reserved_margin` are decremented.
3. The taker **continues matching deeper** in the book — no fill occurs for the self-matched order.

### 6b. Price-banding

The submission-time band ([§3a](#3a-price-banding-for-limit-orders)) only
inspects the price at the moment of placement. Between placement and
matching, the oracle may move far enough that a previously in-band resting
order is now outside the band relative to the current oracle.

To address this, the matching engine applies a **band re-check on every
resting maker** it walks. For each maker encountered during the walk,
the engine evaluates the [§3a](#3a-price-banding-for-limit-orders) band
against the _current_ oracle price. If the maker's resting price is **outside**
the band, it is canceled with `reason = PriceBandViolation`.

## 7. Fill execution

Each fill between taker and maker is executed as follows:

### 7a. Funding settlement

Accrued funding is settled on the user's existing position before the fill:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

The negated accrued funding is added to the user's PnL (positive accrued funding is a cost to longs).

### 7b. Closing PnL

For the closing portion of the fill:

**Long closing (selling to close):**

$$
\mathtt{pnl} = |\mathtt{closingSize}| \times (\mathtt{fillPrice} - \mathtt{entryPrice})
$$

**Short closing (buying to close):**

$$
\mathtt{pnl} = |\mathtt{closingSize}| \times (\mathtt{entryPrice} - \mathtt{fillPrice})
$$

The position size is reduced by the closing amount. If the position is fully closed, it is removed from state.

### 7c. Opening position

For the opening portion of the fill:

- **New position:** entry price is set to the fill price.
- **Existing position (same direction):** entry price is blended as a weighted average:

$$
\mathtt{entryPrice} \gets \frac{|\mathtt{oldSize}| \times \mathtt{oldEntry} + |\mathtt{openingSize}| \times \mathtt{fillPrice}}{|\mathtt{newSize}|}
$$

### 7d. OI update

Open interest is updated per side:

- Closing a long: $\mathtt{longOI} \mathrel{-}= |\mathtt{closingSize}|$
- Closing a short: $\mathtt{shortOI} \mathrel{-}= |\mathtt{closingSize}|$
- Opening a long: $\mathtt{longOI} \mathrel{+}= |\mathtt{openingSize}|$
- Opening a short: $\mathtt{shortOI} \mathrel{+}= |\mathtt{openingSize}|$

## 8. Trading fees

Fees are charged on every fill:

$$
\mathtt{fee} = |\mathtt{fillSize}| \times \mathtt{fillPrice} \times \mathtt{feeRate}
$$

The fee rate differs by role:

| Role  | Rate             | Example value |
| ----- | ---------------- | ------------- |
| Taker | `taker_fee_rate` | 0.1 %         |
| Maker | `maker_fee_rate` | 0 %           |

Fees are always positive (absolute value of fill size is used). They are routed to the vault via the settlement loop described below.

## 9. PnL settlement

After all fills in an order are complete, PnLs and fees are settled atomically as in-place USD margin adjustments. No token conversions occur during settlement — all values are pure `UsdValue` arithmetic.

### 9a. Fee loop

For each non-vault user with a non-zero fee:

$$
\mathtt{userState.margin} \mathrel{-}= \mathtt{fee}
$$

$$
\mathtt{state.vaultMargin} \mathrel{+}= \mathtt{fee}
$$

Fees from the vault to itself are skipped (no-op). Processing fees first ensures collected fees augment $\mathtt{vaultMargin}$ before any vault losses are absorbed.

### 9b. PnL loop

**Non-vault users:**

$$
\mathtt{userState.margin} \mathrel{+}= \mathtt{pnl}
$$

A user's margin can go negative temporarily — the outer function handles bad debt (see [Liquidation](4-liquidation-and-adl.md)).

**Vault:**

$$
\mathtt{vaultMargin} \mathrel{+}= \mathtt{pnl}
$$

A negative $\mathtt{vaultMargin}$ represents a deficit (bad debt not yet recovered via [ADL](4-liquidation-and-adl.md)).

## 10. Unfilled remainder

After matching completes:

- **Market orders and IOC limit orders:** the unfilled remainder is silently discarded. If nothing was filled at all, the transaction reverts with _"no liquidity at acceptable price"_.
- **GTC limit orders:** the unfilled remainder is stored as a resting order. Storage requires:
  - `open_order_count` < `max_open_orders`
  - Price is aligned to the pair's tick size ($\mathtt{limitPrice} \bmod \mathtt{tickSize} = 0$)
  - Sufficient available margin (skipped for reduce-only orders) — see below

**Margin reservation (non-reduce-only):**

The unfilled portion's margin requirement is computed and checked against available margin (see [Margin §7–§8](1-margin.md#7-reserved-margin)):

$$
\mathtt{marginToReserve} = |\mathtt{openingSize}| \times \mathtt{limitPrice} \times \mathtt{imr}
$$

$$
\mathtt{availableMargin} \geq \mathtt{marginToReserve}
$$

If the check passes, `reserved_margin` is increased by $\mathtt{marginToReserve}$ and `open_order_count` is incremented. This is the **0 %-fill scenario** check — it ensures the user can afford the order even if nothing fills immediately.

**Post-only limit orders** take a fast path that bypasses the matching engine entirely. They are rejected if they would cross the best price on the opposite side:

- Buy: $\mathtt{limitPrice} \geq \mathtt{bestAsk}$
- Sell: $\mathtt{limitPrice} \leq \mathtt{bestBid}$

If the opposite book is empty, the order always succeeds.

## 11. Open interest constraint

Each pair has a $\mathtt{maxAbsOI}$ parameter enforcing a per-side cap:

- Long opening: $\mathtt{longOI} + \mathtt{openingSize} \leq \mathtt{maxAbsOI}$
- Short opening: $\mathtt{shortOI} + |\mathtt{openingSize}| \leq \mathtt{maxAbsOI}$

The constraint is checked **before matching** and does not apply to reduce-only orders (which have zero opening size). Long and short OI limits are independent but share the same $\mathtt{maxAbsOI}$ parameter.

## 12. Order cancellation

### Single cancel

A user can cancel any individual resting order by its order ID.

On cancellation:

1. The order is removed from the book.
2. `reserved_margin` is released (subtracted from the user's total).
3. `open_order_count` is decremented.
4. If the user state is now empty (no positions, no open orders, no pending unlocks), it is deleted from storage.

### Bulk cancel

A user can cancel **all** of their resting orders across both sides of the book in a single transaction. The contract iterates the user's resting orders, removing each one and releasing margin. The same cleanup logic applies — if the user state becomes empty after all orders are removed, it is deleted.
