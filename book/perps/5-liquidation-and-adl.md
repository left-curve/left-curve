# Liquidation & Auto-Deleveraging (ADL)

This document describes how the perpetual futures exchange protects itself from
under-collateralised accounts and socialises losses that exceed the vault.

## 1. Liquidation trigger

Every account has an **equity** and a **maintenance margin** (MM):

$$
\mathtt{equity} = \mathtt{collateralValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

$$
\mathtt{MM} = \sum |\mathtt{positionSize}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
$$

where $\mathtt{mmr}$ is the per-pair maintenance-margin ratio. An account becomes
**liquidatable** when

$$
\mathtt{equity} < \mathtt{MM}
$$

Strict inequality: an account whose equity exactly equals its MM is still safe.
An account with no open positions is never liquidatable regardless of its equity.

---

## 2. Close schedule

When an account is liquidatable the system computes the **minimum set of
position closures** needed to restore it above maintenance margin.

1. For every open position, compute its MM contribution:

   $$
   \mathtt{mmContribution} = |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
   $$

2. Sort positions by MM contribution **descending** (largest first).

3. Walk the sorted list and close just enough to cover the deficit:

```text
deficit = MM − equity

for each position (largest MM first):
    if deficit ≤ 0: stop
    close_amount = min(⌈deficit / (oracle_price × mmr)⌉, |size|)
    deficit −= close_amount × oracle_price × mmr
```

This produces a vector of _(pair, close\_size)_ entries. Each close\_size has the
**opposite sign** of the existing position (a long is closed with a sell, a
short with a buy). Only positions that contribute to the deficit are touched and
they may be **partially closed** when the deficit is small relative to the
position.

## 3. Position closure

Each entry in the close schedule is executed in two phases:

### 3a. Order-book matching

The close is submitted as a **market order** against the on-chain order book.
It matches resting limit orders at price-time priority. Any filled amount is
settled normally (mark-to-market PnL between the entry price and the fill
price).

### 3b. Vault backstop

If any quantity remains **unfilled** after the order book is exhausted, the
**vault** absorbs it. Both the liquidated user and the vault are settled at the
current **oracle price** with the vault taking the opposite side. This
guarantees every liquidation completes regardless of order-book depth.

Liquidation fills (both order-book and backstop) carry **zero trading fees** for
both taker and maker.

## 4. Liquidation fee

After all positions in the schedule are closed, a one-time liquidation fee is
charged:

$$
\mathtt{rawFee} = \mathtt{closedNotional} \times \mathtt{liquidationFeeRate}
$$

$$
\mathtt{remainingMargin} = \max(0,\; \mathtt{collateral} + \mathtt{userPnlAfterCloses})
$$

$$
\mathtt{fee} = \min(\mathtt{rawFee},\; \mathtt{remainingMargin})
$$

The fee is deducted from the user's PnL and routed to the **vault**.
It is capped at the remaining margin so the fee itself never creates bad debt.

## 5. PnL settlement

All PnL from the liquidation fills (user, counterparties, vault) is settled
atomically as in-place USD margin adjustments — no token transfers occur. Both
user and vault PnL are applied via the same settlement logic described in
[Order matching §8](2-order-matching.md#8-pnl-settlement).

## 6. Bad debt

If the amount the liquidated user owes exceeds their remaining collateral
balance, bad debt arises:

$$
\mathtt{badDebt} = \mathtt{collections} - \mathtt{collateralBalance}
$$

The bad debt is subtracted from the vault margin:

$$
\mathtt{vaultMargin} \gets \mathtt{vaultMargin} - \mathtt{badDebt}
$$

This can drive $\mathtt{vaultMargin}$ negative. A negative vault margin
represents the **ADL deficit** — bad debt not yet recovered — and triggers
auto-deleveraging.

## 7. ADL trigger

Auto-deleveraging activates whenever

$$
\mathtt{vaultMargin} < 0
$$

This can only happen when a liquidation produces bad debt that exceeds the
vault's available margin. Once active, addresses listed in the `adl_operators`
parameter set may call the deleverage action on profitable accounts until
$\mathtt{vaultMargin}$ is restored to zero or above.

## 8. ADL ranking

Each profitable position is scored to determine closure priority:

$$
\mathtt{pnlPct} = \frac{\mathtt{unrealisedPnl}}{\mathtt{equity}}
$$

$$
\mathtt{leverage} = \frac{\mathtt{notional}}{\mathtt{equity}} \quad \text{where } \mathtt{notional} = |\mathtt{size}| \times \mathtt{oraclePrice}
$$

$$
\mathtt{adlScore} = \mathtt{pnlPct} \times \mathtt{leverage}
$$

Positions with non-positive PnL or non-positive equity score **zero** and are
never selected. Among eligible positions, the **highest score** is closed first.
The score naturally favours accounts that are both highly profitable and highly
leveraged — those who benefited most from the move that caused the bad debt and
who pose the greatest risk if the market reverses.

## 9. ADL closure

All of the target user's profitable positions (score > 0) are closed:

1. Each position is **fully closed** at the **oracle price** with **zero fees**.
2. The close is settled the same way as a normal fill (mark-to-market PnL
   between entry price and oracle price).

Because ADL uses the oracle price, the affected user experiences no slippage
beyond what the oracle already reflects.

## 10. Forfeiture

After ADL closes are settled the user's total realised PnL is applied to the
deficit:

$$
\mathtt{pnl} = \sum \mathtt{realisedPnlFromAdlCloses} \quad (\text{always} > 0 \text{ for selected users})
$$

$$
\mathtt{deficit} = |\mathtt{vaultMargin}|
$$

$$
\mathtt{forfeited} = \min(\mathtt{pnl},\; \mathtt{deficit})
$$

$$
\mathtt{credit} = \mathtt{pnl} - \mathtt{forfeited}
$$

$$
\mathtt{vaultMargin} \gets \mathtt{vaultMargin} + \mathtt{forfeited}
$$

The user forfeits up to the absolute value of the negative $\mathtt{vaultMargin}$
of their profit to make the exchange whole. The remainder ($\mathtt{credit}$) is
added to the user's margin. No collateral beyond the realised PnL is ever
seized.

## Examples

All examples use:

| Parameter                      | Value      |
| ------------------------------ | ---------- |
| Pair                           | BTC / USD  |
| Maintenance-margin ratio (mmr) | 5 %        |
| Liquidation-fee rate           | 0.1 %      |
| Settlement currency            | USDC at $1 |

### Example 1 — Clean liquidation (no bad debt)

**Setup**

|             | Alice      | Bob         |
| ----------- | ---------- | ----------- |
| Direction   | Long 1 BTC | Short 1 BTC |
| Entry price | $50,000    | $50,000     |
| Collateral  | $3,000     | $10,000     |

**BTC drops to $47,500**

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

The long is closed (sold) at the oracle price of $47,500 (order book or vault
backstop).

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

_Settlement_

$$
\text{Alice owes} = \$2{,}500 + \$47.50 = \$2{,}547.50
$$

$$
\text{Alice receives} = \$3{,}000 - \$2{,}547.50 = \$452.50
$$

$$
\mathtt{badDebt} = \$0 \quad (\text{collateral covers everything})
$$

$$
\mathtt{vaultMargin} \mathrel{+}= \$47.50
$$

### Example 2 — Bad debt absorbed by the vault

**Setup**

|              | Charlie    | Dana        |
| ------------ | ---------- | ----------- |
| Direction    | Long 1 BTC | Short 1 BTC |
| Entry price  | $50,000    | $50,000     |
| Collateral   | $3,000     | $10,000     |
| Vault margin | $5,000     |             |

**BTC drops to $46,000**

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

_Execution_

Closed at oracle price $46,000.

$$
\mathtt{CharliePnL} = 1 \times (\$46{,}000 - \$50{,}000) = -\$4{,}000
$$

_Liquidation fee_

$$
\mathtt{remainingMargin} = \max(0,\; \$3{,}000 - \$4{,}000) = \$0
$$

$$
\mathtt{fee} = \min(\text{anything},\; \$0) = \$0
$$

Charlie's equity is already negative so no fee can be collected.

_Settlement and bad debt_

$$
\text{Charlie owes} = \$4{,}000, \quad \text{Charlie collateral} = \$3{,}000
$$

$$
\mathtt{badDebt} = \$4{,}000 - \$3{,}000 = \$1{,}000
$$

$$
\mathtt{absorbed} = \min(\$1{,}000,\; \$5{,}000) = \$1{,}000, \quad \mathtt{unabsorbed} = \$0
$$

$$
\mathtt{vaultMargin}: \$5{,}000 - \$1{,}000 = \$4{,}000
$$

The vault absorbs the full $1,000 shortfall. Charlie's entire $3,000
collateral is collected and no ADL is needed.

### Example 3 — Vault exhausted, ADL triggered

**Setup**

|              | Charlie    | Dana        |
| ------------ | ---------- | ----------- |
| Direction    | Long 1 BTC | Short 1 BTC |
| Entry price  | $50,000    | $50,000     |
| Collateral   | $3,000     | $10,000     |
| Vault margin | $500       |             |

Same positions as Example 2, but the vault is smaller.

**BTC drops to $46,000**

_Charlie's liquidation_ proceeds identically:

$$
\mathtt{CharliePnL} = -\$4{,}000
$$

$$
\mathtt{badDebt} = \$4{,}000 - \$3{,}000 = \$1{,}000
$$

$$
\mathtt{absorbed} = \min(\$1{,}000,\; \$500) = \$500, \quad \mathtt{unabsorbed} = \$500
$$

$$
\mathtt{vaultMargin}: \$500 - \$1{,}000 = -\$500
$$

The vault margin is negative, indicating $500 of unresolved bad debt. ADL activates.

_ADL — selecting Dana_

Dana holds a short that is profitable at $46,000:

$$
\mathtt{unrealisedPnl} = -1 \times (\$46{,}000 - \$50{,}000) = \$4{,}000 \;\text{(profit)}
$$

$$
\mathtt{equity} = \$10{,}000 + \$4{,}000 = \$14{,}000
$$

$$
\mathtt{notional} = 1 \times \$46{,}000 = \$46{,}000
$$

$$
\mathtt{pnlPct} = \frac{\$4{,}000}{\$14{,}000} \approx 0.286
$$

$$
\mathtt{leverage} = \frac{\$46{,}000}{\$14{,}000} \approx 3.286
$$

$$
\mathtt{adlScore} = 0.286 \times 3.286 \approx 0.94
$$

Score is positive, so Dana is eligible for ADL.

_ADL — closing Dana's position_

Dana's short is fully closed at the oracle price of $46,000 with zero fees.

$$
\mathtt{DanaRealisedPnl} = \$4{,}000
$$

_Forfeiture_

$$
\mathtt{deficit} = |\mathtt{vaultMargin}| = \$500
$$

$$
\mathtt{forfeited} = \min(\$4{,}000,\; \$500) = \$500
$$

$$
\mathtt{credit} = \$4{,}000 - \$500 = \$3{,}500
$$

$$
\mathtt{vaultMargin} \gets -\$500 + \$500 = \$0
$$

Dana forfeits $500 of her $4,000 profit to cover the deficit and
receives $3,500. The vault margin is recovered to $0.

**Final state**

|              | Balance                                             |
| ------------ | --------------------------------------------------- |
| Charlie      | $0 (fully liquidated)                               |
| Dana         | $10,000 + $3,500 = $13,500 (profit reduced by $500) |
| Vault margin | $0                                                  |
