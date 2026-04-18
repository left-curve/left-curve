# Liquidation & Auto-Deleveraging (ADL)

This document describes how the perpetual futures exchange protects itself from under-collateralised accounts and socialises losses via auto-deleveraging and the insurance fund.

## 1. Liquidation trigger

Every account has an **equity** and a **maintenance margin** (MM):

$$
\mathtt{equity} = \mathtt{collateralValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

$$
\mathtt{MM} = \sum |\mathtt{positionSize}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
$$

where $\mathtt{mmr}$ is the per-pair maintenance-margin ratio. An account becomes **liquidatable** when

$$
\mathtt{equity} < \mathtt{MM}
$$

Strict inequality: an account whose equity exactly equals its MM is still safe. An account with no open positions is never liquidatable regardless of its equity.

## 2. Close schedule

When an account is liquidatable, the system computes the **minimum set of position closures** needed to restore it above maintenance margin.

1. For every open position, compute its MM contribution:

   $$
   \mathtt{mmContribution} = |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
   $$

2. Sort positions by MM contribution **descending** (largest first).

3. Walk the sorted list and close just enough to cover the deficit:

   - $\mathtt{deficit} = \mathtt{MM} − \mathtt{equity} \;/\; (1 + b)$

     where $b$ is the global `liquidation_buffer_ratio` (default 0). When $b > 0$, positions are closed slightly beyond the maintenance boundary so the user's post-liquidation equity exceeds their remaining MM by a factor of $(1 + b)$, preventing repeated small liquidations from minor adverse price movements.

   - For each position:
     - If $\mathtt{deficit} \le 0$: stop
     - $\mathtt{closeSize} = \min \left( \left\lceil \frac{\mathtt{deficit}}{\mathtt{oraclePrice} \times \mathtt{mmr}} \right\rceil,\; |\mathtt{size}| \right)$
     - $\mathtt{deficit} \mathrel{-}= \mathtt{closeSize} \times \mathtt{oraclePrice} \times \mathtt{mmr}$

This produces a vector of $(\mathtt{pairId},\; \mathtt{closeSize})$ entries. Each $\mathtt{closeSize}$ has the **opposite sign** of the existing position (a long is closed with a sell, a short with a buy). Only positions that contribute to the deficit are touched and they may be **partially closed** when the deficit is small relative to the position.

## 3. Position closure

Each entry in the close schedule is executed in two phases:

### 3a. Order book matching

The close is submitted as a **market order** against the on-chain order book. It matches resting limit orders at price-time priority. Any filled amount is settled normally (mark-to-market PnL between the entry price and the fill price).

### 3b. Auto-deleveraging (ADL)

If any quantity remains **unfilled** after the order book is exhausted, the system automatically deleverages against counter-parties. The unfilled remainder is closed against the **most profitable counter-positions** at the liquidated user's **bankruptcy price**.

**Counter-party selection:** Positions are indexed by the tuple $(\mathtt{pairId},\; \mathtt{entryPrice},\; \mathtt{user})$. For a long being liquidated (selling), the system finds shorts with the highest entry price (most profitable) first. For a short being liquidated (buying), it finds longs with the lowest entry price first.[^1]

[^1]: This does not perfectly rank by total PnL since it ignores accumulated funding fees, but is a reasonable and efficient approximation.

**Bankruptcy price:** The fill price at which the liquidated user's total equity would be exactly zero:

$$
\mathtt{bp} = \mathtt{oraclePrice} - \frac{\mathtt{equity}}{\mathtt{closeAmount}} \quad (\text{for longs})
$$

$$
\mathtt{bp} = \mathtt{oraclePrice} + \frac{\mathtt{equity}}{\mathtt{closeAmount}} \quad (\text{for shorts})
$$

Since equity is typically negative for liquidatable users, the bankruptcy price is worse than the oracle price — the counter-party receives a favourable fill. The counter-party's resting limit orders are not affected by ADL; only their position is force-reduced.

Liquidation fills (both order-book and ADL) carry **zero trading fees** for both taker and maker.

Order-book fills during liquidation emit `order_filled` events with a `fill_id` just like regular matches (see the [events reference](8-api.md#9-events-reference)); ADL fills do _not_ — they use the separate `deleveraged` and `liquidated` events, which carry no `fill_id`, because ADL is a position transfer at the bankruptcy price rather than an order-book match.

## 4. Liquidation fee

After all positions in the schedule are closed, a one-time liquidation fee is charged:

$$
\mathtt{rawFee} = \mathtt{closedNotional} \times \mathtt{liquidationFeeRate}
$$

$$
\mathtt{remainingMargin} = \max (0,\; \mathtt{margin} + \mathtt{userPnlAfterCloses})
$$

$$
\mathtt{fee} = \min (\mathtt{rawFee},\; \mathtt{remainingMargin})
$$

The fee is deducted from the user's margin and routed to the **insurance fund** (not the vault). It is capped at the remaining margin so the fee itself never creates bad debt.

## 5. PnL settlement

All PnL from the liquidation fills (user, book makers, ADL counter-parties) is settled atomically as in-place USD margin adjustments — no token transfers occur. Both user and maker PnL are applied via the same settlement logic described in [Order matching §8](2-order-matching.md#8-pnl-settlement).

## 6. Bad debt

After PnL and fee settlement, if the user's margin is negative the absolute value is bad debt. The margin is floored to zero and the bad debt is subtracted from the **insurance fund**:

$$
\mathtt{badDebt} = |\min(0,\; \mathtt{margin\ after\ settlement})|
$$

$$
\mathtt{user.margin} \gets 0
$$

$$
\mathtt{insuranceFund} \mathrel{-}= \mathtt{badDebt}
$$

The insurance fund may go negative. A negative insurance fund represents unresolved bad debt — future liquidation fees will replenish it.

Note: when positions are fully ADL'd at the bankruptcy price, the user's equity is zeroed by construction. Bad debt from ADL fills is therefore zero. Bad debt arises only from **book fills** at prices worse than the bankruptcy price (e.g., thin order books with deep bids/asks far from oracle).

## 7. Insurance fund

The insurance fund is a separate pool from the vault that absorbs bad debt and is funded by liquidation fees.

**Funding:** Every liquidation fee ([§4](#4-liquidation-fee)) is credited to the insurance fund.

**Usage:** Every bad debt event ([§6](#6-bad-debt)) is debited from the insurance fund.

**Negative balance:** The insurance fund may go negative when accumulated bad debt exceeds accumulated fees. This is the simplest approach — no special trigger or intervention is needed. Future liquidation fees will naturally replenish the fund.

Other users' bad debt and liquidation fees never touch the vault's margin — this isolates liquidity providers from external liquidation losses. However, the vault itself is subject to liquidation like any other account. If the vault's equity falls below its maintenance margin, its positions are closed following the same procedure described above. The vault's own liquidation fee goes to the insurance fund, and any bad debt is absorbed by it.

## Examples

All examples use:

| Parameter                      | Value       |
| ------------------------------ | ----------- |
| Pair                           | BTC / USD   |
| Maintenance-margin ratio (mmr) | 5 %         |
| Liquidation-fee rate           | 0.1 %       |
| Settlement currency            | USDC at \$1 |

### Example 1 — Clean liquidation on book (no bad debt)

**Setup**

|             | Alice      | Bob (maker)          |
| ----------- | ---------- | -------------------- |
| Direction   | Long 1 BTC | Bid 1 BTC @ \$47,500 |
| Entry price | \$50,000   | —                    |
| Margin      | \$3,000    | \$10,000             |

**BTC drops to \$47,500**

_Alice's account_

$$
\mathtt{equity} = \$3{,}000 + 1 \times (\$47{,}500 - \$50{,}000) = \$3{,}000 - \$2{,}500 = \$500
$$

$$
\mathtt{MM} = 1 \times \$47{,}500 \times 5\% = \$2{,}375
$$

$$
\$500 < \$2{,}375 \;\Rightarrow\; \text{liquidatable}
$$

_Close schedule_

Alice has one position; the full 1 BTC long is scheduled for closure.

_Execution_

The long is closed (sold) into Bob's resting bid at \$47,500.

$$
\mathtt{AlicePnL} = 1 \times (\$47{,}500 - \$50{,}000) = -\$2{,}500
$$

_Liquidation fee_

$$
\mathtt{closedNotional} = 1 \times \$47{,}500 = \$47{,}500
$$

$$
\mathtt{rawFee} = \$47{,}500 \times 0.1\% = \$47.50
$$

$$
\mathtt{remainingMargin} = \max(0,\; \$3{,}000 - \$2{,}500) = \$500
$$

$$
\mathtt{fee} = \min(\$47.50,\; \$500) = \$47.50
$$

_Settlement (margin arithmetic)_

Alice's margin starts at \$3,000.

$$
\mathtt{margin} \mathrel{-}= \$47.50 \quad (\text{fee}) \;\Rightarrow\; \$2{,}952.50
$$

$$
\mathtt{margin} \mathrel{+}= (-\$2{,}500) \quad (\text{PnL}) \;\Rightarrow\; \$452.50
$$

Final margin is positive — no bad debt.

$$
\mathtt{insuranceFund} \mathrel{+}= \$47.50 \quad (\text{fee revenue})
$$

### Example 2 — ADL at bankruptcy price (no book liquidity)

**Setup**

|             | Charlie    | Dana        |
| ----------- | ---------- | ----------- |
| Direction   | Long 1 BTC | Short 1 BTC |
| Entry price | \$50,000   | \$55,000    |
| Margin      | \$3,000    | \$10,000    |

**BTC drops to \$46,000**

_Charlie's account_

$$
\mathtt{equity} = \$3{,}000 + 1 \times (\$46{,}000 - \$50{,}000) = \$3{,}000 - \$4{,}000 = -\$1{,}000
$$

$$
\mathtt{MM} = 1 \times \$46{,}000 \times 5\% = \$2{,}300
$$

$$
-\$1{,}000 < \$2{,}300 \;\Rightarrow\; \text{liquidatable}
$$

_Close schedule_

Charlie's full 1 BTC long is scheduled for closure.

_Order book matching_

No bids on the book — the full 1 BTC is unfilled.

_ADL_

Bankruptcy price for Charlie's long:

$$
\mathtt{bp} = \$46{,}000 - \frac{-\$1{,}000}{1} = \$46{,}000 + \$1{,}000 = \$47{,}000
$$

Dana holds the most profitable short (entry \$55,000, current oracle \$46,000). Her position is force-closed at \$47,000.

Charlie's PnL at bankruptcy price:

$$
\mathtt{CharliePnL} = 1 \times (\$47{,}000 - \$50{,}000) = -\$3{,}000
$$

$$
\mathtt{margin}: \$3{,}000 + (-\$3{,}000) = \$0 \quad \text{(zeroed by construction)}
$$

Dana's PnL at bankruptcy price:

$$
\mathtt{DanaPnL} = -1 \times (\$47{,}000 - \$55{,}000) = \$8{,}000
$$

$$
\mathtt{DanaMargin}: \$10{,}000 + \$8{,}000 = \$18{,}000
$$

_Liquidation fee_

$$
\mathtt{remainingMargin} = \max(0,\; \$3{,}000 - \$3{,}000) = \$0
$$

$$
\mathtt{fee} = \$0 \quad \text{(no margin left)}
$$

No bad debt, no insurance fund impact. Dana receives the full PnL at the bankruptcy price, which is better than the oracle price for her.

**Final state**

|                | Balance                 |
| -------------- | ----------------------- |
| Charlie        | \$0 (fully liquidated)  |
| Dana           | \$18,000 (profit at bp) |
| Insurance fund | unchanged               |

### Example 3 — Book fill creates bad debt

**Setup**

|                | Charlie    | Bob (maker)          |
| -------------- | ---------- | -------------------- |
| Direction      | Long 1 BTC | Bid 1 BTC @ \$46,000 |
| Entry price    | \$50,000   | —                    |
| Margin         | \$3,000    | \$50,000             |
| Insurance fund | \$500      |                      |

**BTC drops to \$46,000**

_Charlie's liquidation_

Same equity and MM as Example 2. Liquidatable.

_Order book matching_

The bid at \$46,000 fills Charlie's full 1 BTC sell.

$$
\mathtt{CharliePnL} = 1 \times (\$46{,}000 - \$50{,}000) = -\$4{,}000
$$

_Liquidation fee_

$$
\mathtt{remainingMargin} = \max(0,\; \$3{,}000 - \$4{,}000) = \$0
$$

$$
\mathtt{fee} = \$0
$$

_Bad debt_

Charlie's margin after PnL: $\$3{,}000 - \$4{,}000 = -\$1{,}000$.

$$
\mathtt{badDebt} = \$1{,}000, \quad \mathtt{margin} \gets \$0
$$

$$
\mathtt{insuranceFund}: \$500 - \$1{,}000 = -\$500
$$

The insurance fund goes negative. Future liquidation fees will replenish it.

**Final state**

|                | Balance                          |
| -------------- | -------------------------------- |
| Charlie        | \$0 (fully liquidated)           |
| Insurance fund | −\$500 (unresolved bad debt)     |
| Vault          | unchanged (isolated from losses) |
