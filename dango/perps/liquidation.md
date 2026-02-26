# Liquidation & Auto-Deleveraging (ADL)

This document describes how the perpetual futures exchange protects itself from
under-collateralised accounts and socialises losses that exceed the insurance
fund.

## 1 Liquidation trigger

Every account has an **equity** and a **maintenance margin** (MM):

```plain
equity = collateral_value + Σ unrealised_pnl − Σ accrued_funding

MM = Σ |position_size| × oracle_price × mmr
```

where _mmr_ is the per-pair maintenance-margin ratio. An account becomes
**liquidatable** when

```plain
equity < MM
```

Strict inequality: an account whose equity exactly equals its MM is still safe.
An account with no open positions is never liquidatable regardless of its equity.

---

## 2 Close schedule

When an account is liquidatable the system computes the **minimum set of
position closures** needed to restore it above maintenance margin.

1. For every open position, compute its MM contribution:

   ```plain
   mm_contribution = |size| × oracle_price × mmr
   ```

2. Sort positions by MM contribution **descending** (largest first).

3. Walk the sorted list and close just enough to cover the deficit:

   ```plain
   deficit = MM − equity

   for each position (largest MM first):
       if deficit ≤ 0: stop
       close_amount = min(⌈deficit / (oracle_price × mmr)⌉, |size|)
       deficit −= close_amount × oracle_price × mmr
   ```

This produces a vector of _(pair, close_size)_ entries. Each close_size has the
**opposite sign** of the existing position (a long is closed with a sell, a
short with a buy). Only positions that contribute to the deficit are touched and
they may be **partially closed** when the deficit is small relative to the
position.

## 3 Position closure

Each entry in the close schedule is executed in two phases:

### 3a Order-book matching

The close is submitted as a **market order** against the on-chain order book.
It matches resting limit orders at price-time priority. Any filled amount is
settled normally (mark-to-market PnL between the entry price and the fill
price).

### 3b Vault backstop

If any quantity remains **unfilled** after the order book is exhausted, the
**vault** absorbs it. Both the liquidated user and the vault are settled at the
current **oracle price** with the vault taking the opposite side. This
guarantees every liquidation completes regardless of order-book depth.

Liquidation fills (both order-book and backstop) carry **zero trading fees** for
both taker and maker.

## 4 Liquidation fee

After all positions in the schedule are closed, a one-time liquidation fee is
charged:

```plain
raw_fee = closed_notional × liquidation_fee_rate

remaining_margin = max(0, collateral + user_pnl_after_closes)

fee = min(raw_fee, remaining_margin)
```

The fee is deducted from the user's PnL and routed to the **insurance fund**.
It is capped at the remaining margin so the fee itself never creates bad debt.

## 5 PnL settlement

All PnL from the liquidation fills (user, counterparties, vault) is settled
atomically:

- **Positive PnL** → the user (or vault) receives a payout.
- **Negative PnL** → the user owes a collection.

The vault's PnL is applied directly to a dedicated vault-margin balance rather
than transferred as tokens.

## 6 Bad debt

If the amount the liquidated user owes exceeds their remaining collateral
balance, bad debt arises:

```plain
bad_debt = collections − collateral_balance
```

The insurance fund absorbs as much as it can:

```plain
absorbed  = min(bad_debt, insurance_fund)
unabsorbed = bad_debt − absorbed

insurance_fund −= absorbed
adl_deficit    += unabsorbed
```

If the insurance fund fully covers the bad debt, the system returns to normal.
If not, the unabsorbed amount is recorded as the **ADL deficit** and triggers
auto-deleveraging.

## 7 ADL trigger

Auto-deleveraging activates whenever

```plain
adl_deficit > 0
```

This can only happen when a liquidation produces bad debt that exceeds the
insurance fund. Once active, any third party may call the deleverage action on
profitable accounts until the deficit is cleared.

## 8 ADL ranking

Each profitable position is scored to determine closure priority:

```plain
pnl_pct   = unrealised_pnl / equity
leverage   = notional / equity          (where notional = |size| × oracle_price)

adl_score = pnl_pct × leverage
```

Positions with non-positive PnL or non-positive equity score **zero** and are
never selected. Among eligible positions, the **highest score** is closed first.
The score naturally favours accounts that are both highly profitable and highly
leveraged—those who benefited most from the move that caused the bad debt and
who pose the greatest risk if the market reverses.

## 9 ADL closure

All of the target user's profitable positions (score > 0) are closed:

1. Each position is **fully closed** at the **oracle price** with **zero fees**.
2. The close is settled the same way as a normal fill (mark-to-market PnL
   between entry price and oracle price).

Because ADL uses the oracle price, the affected user experiences no slippage
beyond what the oracle already reflects.

## 10 Forfeiture

After ADL closes are settled the user's total realised PnL is applied to the
deficit:

```plain
pnl = Σ realised_pnl_from_adl_closes          (always > 0 for selected users)

insurance_fund += pnl                          # restock
forfeited       = min(pnl, adl_deficit)
payout          = pnl − forfeited

insurance_fund −= payout
adl_deficit    −= forfeited
```

The user forfeits up to _adl_deficit_ of their profit to make the exchange
whole. The remainder is paid out normally. No collateral beyond the realised PnL
is ever seized.

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

```plain
equity = $3,000 + 1 × ($47,500 − $50,000) = $3,000 − $2,500 = $500
MM     = 1 × $47,500 × 5% = $2,375

$500 < $2,375  →  liquidatable
```

_Close schedule_

Alice has one position; the full 1 BTC long is scheduled for closure.

_Execution_

The long is closed (sold) at the oracle price of $47,500 (order book or vault
backstop).

```plain
Alice PnL = 1 × ($47,500 − $50,000) = −$2,500
```

_Liquidation fee_

```plain
closed notional  = 1 × $47,500 = $47,500
raw fee          = $47,500 × 0.1% = $47.50
remaining margin = max(0, $3,000 − $2,500) = $500

fee = min($47.50, $500) = $47.50
```

_Settlement_

```plain
Alice owes        = $2,500 + $47.50 = $2,547.50
Alice collateral  = $3,000
Alice receives    = $3,000 − $2,547.50 = $452.50

Bad debt = $0      (collateral covers everything)
Insurance fund    += $47.50
```

### Example 2 — Bad debt absorbed by the insurance fund

**Setup**

|                | Charlie    | Dana        |
| -------------- | ---------- | ----------- |
| Direction      | Long 1 BTC | Short 1 BTC |
| Entry price    | $50,000    | $50,000     |
| Collateral     | $3,000     | $10,000     |
| Insurance fund | $5,000     |             |

**BTC drops to $46,000**

_Charlie's account_

```plain
equity = $3,000 + 1 × ($46,000 − $50,000) = $3,000 − $4,000 = −$1,000
MM     = 1 × $46,000 × 5% = $2,300

−$1,000 < $2,300  →  liquidatable
```

_Close schedule_

Charlie's full 1 BTC long is scheduled for closure.

_Execution_

Closed at oracle price $46,000.

```plain
Charlie PnL = 1 × ($46,000 − $50,000) = −$4,000
```

_Liquidation fee_

```plain
remaining margin = max(0, $3,000 − $4,000) = $0

fee = min(anything, $0) = $0
```

Charlie's equity is already negative so no fee can be collected.

_Settlement and bad debt_

```plain
Charlie owes       = $4,000
Charlie collateral = $3,000
bad debt           = $4,000 − $3,000 = $1,000

absorbed   = min($1,000, $5,000) = $1,000
unabsorbed = $0

Insurance fund: $5,000 − $1,000 = $4,000
ADL deficit:    $0
```

The insurance fund absorbs the full $1,000 shortfall. Charlie's entire $3,000
collateral is collected and no ADL is needed.

### Example 3 — Insurance fund exhausted, ADL triggered

**Setup**

|                | Charlie    | Dana        |
| -------------- | ---------- | ----------- |
| Direction      | Long 1 BTC | Short 1 BTC |
| Entry price    | $50,000    | $50,000     |
| Collateral     | $3,000     | $10,000     |
| Insurance fund | $500       |             |

Same positions as Example 2, but the insurance fund is smaller.

**BTC drops to $46,000**

_Charlie's liquidation_ proceeds identically:

```plain
Charlie PnL    = −$4,000
bad debt       = $4,000 − $3,000 = $1,000

absorbed       = min($1,000, $500) = $500
unabsorbed     = $500

Insurance fund: $500 − $500 = $0
ADL deficit:    $500
```

The insurance fund is exhausted with $500 of bad debt remaining. ADL activates.

_ADL — selecting Dana_

Dana holds a short that is profitable at $46,000:

```plain
Dana unrealised PnL = −1 × ($46,000 − $50,000) = $4,000  (profit)
Dana equity         = $10,000 + $4,000 = $14,000
notional            = 1 × $46,000 = $46,000

pnl_pct  = $4,000 / $14,000  ≈ 0.286
leverage = $46,000 / $14,000 ≈ 3.286

adl_score = 0.286 × 3.286 ≈ 0.94
```

Score is positive, so Dana is eligible for ADL.

_ADL — closing Dana's position_

Dana's short is fully closed at the oracle price of $46,000 with zero fees.

```plain
Dana realised PnL = $4,000
```

_Forfeiture_

```plain
insurance_fund += $4,000            →  $4,000
forfeited       = min($4,000, $500) =  $500
payout          = $4,000 − $500     =  $3,500

insurance_fund −= $3,500            →  $500
adl_deficit    −= $500              →  $0
```

Dana forfeits $500 of her $4,000 profit to cover the remaining deficit and
receives $3,500. The insurance fund is restocked to $500 and the ADL deficit is
cleared.

**Final state**

|                | Balance                                             |
| -------------- | --------------------------------------------------- |
| Charlie        | $0 (fully liquidated)                               |
| Dana           | $10,000 + $3,500 = $13,500 (profit reduced by $500) |
| Insurance fund | $500                                                |
| ADL deficit    | $0                                                  |
