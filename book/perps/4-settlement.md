# Settlement

## 1 Overview

The **bank contract** is the system-level token ledger. It holds all user
balances and processes every transfer. For the perps contract it enforces two
special rules:

1. **Margin check on transfer** — prevents users with open perps positions from
   withdrawing collateral that backs those positions.
2. **Perps overdraw** — allows the perps contract to send settlement currency it
   does not yet hold, tracking the shortfall as a deficit.

## 2 Margin check on transfer

When a user transfers settlement currency and has open perps positions, the
bank queries the perps contract to verify the transfer does not breach margin
requirements.

The check triggers only when **all three conditions** hold:

1. The transfer includes settlement currency.
2. The sender has a user state in the perps contract.
3. That user state has at least one open position.

### Available margin

$$
\mathtt{equity} = \mathtt{collateralValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

$$
\mathtt{usedMargin} = \sum |\mathtt{positionSize}| \times \mathtt{oraclePrice} \times \mathtt{imr}
$$

$$
\mathtt{availableMargin} = \max\!\left(0,\; \mathtt{equity} - \mathtt{usedMargin} - \mathtt{reservedMargin}\right)
$$

where $\mathtt{imr}$ is the initial margin ratio and $\mathtt{reservedMargin}$
is margin locked for resting limit orders.

The available margin is converted back to settlement-currency base units using
**floor** rounding (conservative — the user gets slightly less):

$$
\mathtt{availableBase} = \left\lfloor \frac{\mathtt{availableMargin}}{\mathtt{settlementPrice}} \times 10^{\mathtt{decimal}} \right\rfloor
$$

The transfer is **rejected** if:

$$
\mathtt{transferAmount} > \mathtt{availableBase}
$$

## 3 Perps overdraw

In perpetual futures, whenever there is a winner there is necessarily a loser.
When the winner realises positive PnL, the perps contract must pay out
settlement currency from its balance. But the corresponding loser may not have
realised their loss yet — they may still hold the position open.

This creates a timing mismatch: the contract may need to send tokens it does
not yet hold. Rather than blocking the winner's payout, the bank allows the
perps contract to **overdraw** its settlement-currency balance. The shortfall
is tracked in a dedicated storage slot:

$$
\mathtt{PERP\_DEFICIT} \in \mathbb{Z}_{\geq 0}
$$

## 4 `decrease_perp_balance`

When the perps contract sends settlement currency, the bank uses a special
balance-decrease function that absorbs as much as possible from the actual
balance and records the remainder as deficit:

$$
\mathtt{absorbed} = \min(\mathtt{amount},\; \mathtt{balance})
$$

$$
\mathtt{balance} \gets \mathtt{balance} - \mathtt{absorbed}
$$

$$
\mathtt{deficit} \gets \mathtt{deficit} + (\mathtt{amount} - \mathtt{absorbed})
$$

This function **never errors** due to insufficient balance. It always succeeds,
recording any shortfall. If the resulting balance is zero the storage entry is
deleted; if the deficit is updated it is saved.

## 5 `increase_perp_balance`

When the perps contract receives settlement currency (e.g. a loser realising
their loss), the bank repays the deficit **first** before crediting the
balance:

$$
\mathtt{absorbed} = \min(\mathtt{amount},\; \mathtt{deficit})
$$

$$
\mathtt{deficit} \gets \mathtt{deficit} - \mathtt{absorbed}
$$

$$
\mathtt{balance} \gets \mathtt{balance} + (\mathtt{amount} - \mathtt{absorbed})
$$

If the new deficit is zero the storage entry is removed entirely.

## 6 Invariant

The deficit is **temporary by design**. Every winner's profit has a
corresponding loser with an equal-and-opposite unrealised PnL. That loser must
eventually realise their loss — either by closing voluntarily or by being
[liquidated](liquidation-and-adl.md). When they do, settlement currency flows
back to the perps contract via `increase_perp_balance`, repaying the deficit.

The guarantee rests on two properties:

1. Every winner implies a loser with matching unrealised loss.
2. Losers are **forced** to close via liquidation when their equity falls below
   the maintenance margin.

Therefore $\mathtt{PERP\_DEFICIT}$ is bounded by the aggregate unrealised losses
of all losing positions, and those positions are guaranteed to close eventually.
