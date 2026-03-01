# Margin

## 1 Overview

All trader margin is held **internally** in the perps contract as a USD value
(`UsdValue`) on each user's state. PnL and fee settlement is pure USD
arithmetic — no token conversions are needed during matching or liquidation.

Token conversion only happens at two boundaries:

- **Deposit** — the user sends settlement currency (USDC) to the perps
  contract; the oracle price converts the token amount to USD and credits
  `userState.margin`.
- **Withdraw** — the user requests a USD amount; the oracle price converts it
  to settlement-currency tokens (floor-rounded) and transfers them out.

Internal logics of the perps contract use USD amounts exclusively.

## 2 Trader Deposit

The user sends settlement currency as attached funds. The perps contract:

1. Queries the oracle for the settlement-currency price.
2. Converts the token amount to USD: $\mathtt{depositValue} = \mathtt{amount} \times \mathtt{price}$.
3. Credits `userState.margin` by $\mathtt{depositValue}$.

The tokens remain in the perps contract's bank balance.

## 3 Trader Withdraw

The user specifies how much USD margin to withdraw. The perps contract:

1. Computes available margin (equity minus used margin minus reserved margin;
   see [§8](#8-available-margin)),
   clamped to zero.
2. Ensures the requested amount does not exceed available margin.
3. Deducts the amount from `userState.margin`.
4. Converts USD to settlement-currency tokens at the current oracle price
   (floor-rounded for safety — the contract keeps slightly more than strictly
   needed).
5. Transfers the tokens to the user.

## 4 Equity

A user's **equity** (net account value) is:

$$
\mathtt{equity} = \mathtt{collateralValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

where `collateralValue` is the USD value of the user's deposited margin
(`userState.margin`). Per-position unrealised PnL is:

$$
\mathtt{unrealisedPnl} = \mathtt{size} \times (\mathtt{oraclePrice} - \mathtt{entryPrice})
$$

and accrued funding is:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

Positive accrued funding is a cost to the trader (subtracted from equity).
Refer to [Funding](3-funding.md) for details on the funding rate.

## 5 Initial margin (IM)

$$
\mathtt{IM} = \sum |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{imr}
$$

where `imr` is the per-pair `initial_margin_ratio`. IM is the minimum equity
required to **open or hold** positions. It is used in two places:

- **Pre-match margin check** — verifies the taker can afford the worst-case
  100 % fill (see [Order matching §4a](2-order-matching.md#4a-pre-match-margin-check)).
- **Available margin calculation** — determines how much can be withdrawn or
  committed to new limit orders (see [§8](#8-available-margin) below).

When checking a new order the IM is computed with a **projected** size: the
user's current position in that pair is replaced by the hypothetical
post-fill position (`currentSize + orderSize`). Positions in other pairs use
their actual sizes.

## 6 Maintenance margin (MM)

$$
\mathtt{MM} = \sum |\mathtt{size}| \times \mathtt{oraclePrice} \times \mathtt{mmr}
$$

where `mmr` is the per-pair `maintenance_margin_ratio` (always ≤ `imr`).
A user becomes eligible for liquidation when:

$$
\mathtt{equity} < \mathtt{MM}
$$

See [Liquidation](5-liquidation-and-adl.md) for details.

## 7 Reserved margin

When a GTC limit order is placed, margin is reserved for the worst-case
scenario (the entire order is opening):

$$
\mathtt{reservedPerOrder} = |\mathtt{openingSize}| \times \mathtt{limitPrice} \times \mathtt{imr}
$$

The user's total `reservedMargin` is the sum across all resting orders.
Reserved margin is released proportionally as orders fill and fully released
on cancellation. Reduce-only orders reserve zero margin (they can only close).

See [Order matching §9](2-order-matching.md#9-unfilled-remainder) for when
reservation occurs.

## 8 Available margin

$$
\mathtt{available} = \max\!\big(0,\; \mathtt{equity} - \mathtt{usedMargin} - \mathtt{reservedMargin}\big)
$$

where `usedMargin` is the IM of current positions (§5 formula applied to
actual sizes, without any projection). This determines how much can be
withdrawn (§3) or committed to new limit orders.
