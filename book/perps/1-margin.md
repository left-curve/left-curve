# Margin

## 1. Overview

All trader margin is held **internally** in the perps contract as a USD value on each user's `userState`.

Internal logics of the perps contract use USD amounts exclusively. Token conversion only happens at two boundaries:

- **Deposit** — the user sends settlement currency (USDC) to the perps contract; the oracle price converts the token amount to USD and credits `userState.margin`.
- **Withdraw** — the user requests a USD amount; the oracle price converts it to settlement currency tokens (floor-rounded) and transfers them out.

## 2. Trader Deposit

The user sends settlement currency as attached funds. The perps contract:

1. Values the settlement currency at a fixed **$1 per unit** (no oracle lookup).
2. Converts the token amount to USD: $\mathtt{depositValue} = \mathtt{amount} \times 1$.
3. Increment `userState.margin` by $\mathtt{depositValue}$.

The tokens remain in the perps contract's bank balance.

## 3. Trader Withdraw

The user specifies how much USD margin to withdraw. The perps contract:

1. Computes $\mathtt{availableMargin}$ (see [§8](#8-available-margin)), clamped to zero.
2. Ensures the requested amount does not exceed $\mathtt{availableMargin}$.
3. Deducts the amount from `userState.margin`.
4. Converts USD to settlement currency tokens at the fixed $1 rate (floor-rounded to base units).
5. Transfers the tokens to the user.

## 4. Equity

A user's **equity** (net account value) is:

$$
\mathtt{equity} = \mathtt{collateralValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

where $\mathtt{collateralValue}$ is the USD value of the user's deposited margin (`userState.margin`).

Per-position unrealised PnL is:

$$
\mathtt{unrealisedPnl} = \mathtt{size} \times (\mathtt{oraclePrice} - \mathtt{entryPrice})
$$

and accrued funding is:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

Positive accrued funding is a cost to the trader (subtracted from equity). Refer to [Funding](3-funding.md) for details on the funding rate.

## 5. Initial margin (IM)

$$
\mathtt{IM} = \sum |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{imr}
$$

where $\mathtt{imr}$ is the per-pair **initial margin ratio**. IM is the minimum equity required to **open or hold** positions. It is used in two places:

- **Pre-match margin check** — verifies the taker can afford the worst-case 100 % fill (see [Order matching §5](2-order-matching.md#5-pre-match-margin-check)).
- **Available margin calculation** — determines how much can be withdrawn or committed to new limit orders (see [§8](#8-available-margin) below).

When checking a new order the IM is computed with a **projected** size: the user's current position in that pair is replaced by the hypothetical post-fill position ($\mathtt{currentSize} + \mathtt{orderSize}$). Positions in other pairs use their actual sizes.

## 6. Maintenance margin (MM)

$$
\mathtt{MM} = \sum |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
$$

where $\mathtt{mmr}$ is the per-pair **maintenance margin ratio** (always $\le \mathtt{imr}$). A user becomes eligible for liquidation when:

$$
\mathtt{equity} < \mathtt{MM}
$$

See [Liquidation](4-liquidation-and-adl.md) for details.

## 7. Reserved margin

When a GTC limit order is placed, margin is reserved for the worst-case scenario (the entire order is opening):

$$
\mathtt{reservedPerOrder} = |\mathtt{openingSize}| \times \mathtt{limitPrice} \times \mathtt{imr}
$$

The user's total $\mathtt{reservedMargin}$ is the sum across all resting orders. Reserved margin is released proportionally as orders fill and fully released on cancellation. Reduce-only orders reserve zero margin (they can only close).

See [Order matching §10](2-order-matching.md#10-unfilled-remainder) for when reservation occurs.

## 8. Available margin

$$
\mathtt{available} = \max \big(0,\; \mathtt{equity} - \mathtt{usedMargin} - \mathtt{reservedMargin}\big)
$$

where $\mathtt{usedMargin}$ is the IM of current positions ([§5](#5-initial-margin-im) formula applied to actual sizes, without any projection). This determines how much can be withdrawn ([§3](#3-trader-withdraw)) or committed to new limit orders.
