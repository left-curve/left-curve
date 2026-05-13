# Vault

## 1. Overview

The vault is the **passive market maker** for the perpetual futures exchange. It continuously quotes bid/ask orders around the oracle price on every pair, earning the spread.

Liquidity providers (LPs) deposit settlement currency into the vault and receive vault shares credited to their account.

## 2. Liquidity provision

Adding liquidity follows an **ERC-4626 virtual shares** pattern to prevent the first depositor inflation attack.

### Constants

| Name           | Value     |
| -------------- | --------- |
| Virtual shares | 1,000,000 |
| Virtual assets | \$1       |

### Share minting

The LP specifies a USD margin amount $\mathtt{depositMargin}$ to transfer from their trading margin to the vault.

$$
\mathtt{effectiveSupply} = \mathtt{vaultShareSupply} + \mathtt{virtualShares}
$$

$$
\mathtt{effectiveEquity} = \mathtt{vaultEquity} + \mathtt{virtualAssets}
$$

$$
\mathtt{sharesToMint} = \left\lfloor \mathtt{effectiveSupply} \times \frac{\mathtt{depositMargin}}{\mathtt{effectiveEquity}} \right\rfloor
$$

Floor rounding protects the vault from rounding exploitation. A minimum-shares parameter lets depositors revert if slippage is too high.

### First depositor protection

The virtual terms dominate when real supply and equity are small. An attacker cannot inflate the share price to steal from subsequent depositors because the initial share price is effectively $\$1 / 1{,}000{,}000 = \$0.000001$ per share.

## 3. Liquidity withdrawal

The LP specifies how many vault shares to burn. The USD value to release is computed:

$$
\mathtt{releaseValue} = \mathtt{effectiveEquity} \times \frac{\mathtt{sharesToBurn}}{\mathtt{effectiveSupply}}
$$

The fund is not released immediately. A cooldown is initiated, with the ending time computed as:

$$
\mathtt{endTime} = \mathtt{currentTime} + \mathtt{vaultCooldownPeriod}
$$

Once $\mathtt{endTime}$ is reached, the contract credits the released USD value back to the LP's trading margin.

## 4. Vault equity

The vault has its own user state (positions acquired from market-making fills). Its equity follows the same formula as any user:

$$
\mathtt{vaultEquity} = \mathtt{vaultMarginValue} + \sum \mathtt{unrealisedPnl} - \sum \mathtt{accruedFunding}
$$

where $\mathtt{vaultMargin}$ is the vault's internal USD margin (updated in-place during settlement), and the sums run over all of the vault's open positions.

If $\mathtt{effectiveEquity}$ is non-positive the vault is in catastrophic loss and both deposits and withdrawals are disabled.

## 5. Market making policy

The vault uses its margin to market make in the order book. Each block, after the oracle update, the vault cancels all existing quotes and recomputes bid/ask orders for every pair.

The strategy uses **inventory skew** to reduce the vault's exposure to directional price movements. When the vault accumulates a position in one direction, it tilts both order sizes and spreads to encourage trades that unwind that position.

### Margin allocation

Total vault **available margin** is split across pairs by weight:

$$
\mathtt{pairMargin} = \mathtt{vaultAvailableMargin} \times \frac{\mathtt{vaultLiquidityWeight}}{\mathtt{vaultTotalWeight}}
$$

where $\mathtt{vaultAvailableMargin} = \max(0,\; \mathtt{equity} - \mathtt{usedMargin})$ and $\mathtt{usedMargin}$ is the sum of initial margin across all vault positions (see [Margin §8](1-margin.md#8-available-margin)).

### Skew ratio

For each pair, compute a skew ratio from the vault's current position:

$$
\mathtt{skew} = \mathrm{clamp}\!\left(\frac{\mathtt{positionSize}}{\mathtt{maxSkewSize}},\; -1,\; 1\right)
$$

where $\mathtt{positionSize}$ is the vault's signed position (positive = long, negative = short) and $\mathtt{maxSkewSize}$ is the position size at which skew saturates.

At zero position, $\mathtt{skew} = 0$ and quoting is symmetric. At maximum long, $\mathtt{skew} = 1$. At maximum short, $\mathtt{skew} = -1$.

### Quote size

Each side receives half the allocated margin, capped by a per-pair maximum, then tilted by the skew:

$$
\mathtt{halfMargin} = \frac{\mathtt{pairMargin}}{2}
$$

$$
\mathtt{baseSize} = \min \left(\frac{\mathtt{halfMargin}}{\mathtt{oraclePrice} \times \mathtt{imr}},\; \mathtt{maxQuoteSize}\right)
$$

$$
\mathtt{bidSize} = \mathtt{baseSize} \times (1 - \mathtt{skew} \times \mathtt{sizeSkewFactor})
$$

$$
\mathtt{askSize} = \mathtt{baseSize} \times (1 + \mathtt{skew} \times \mathtt{sizeSkewFactor})
$$

where $\mathtt{imr}$ is the initial margin ratio and $\mathtt{sizeSkewFactor} \in [0, 1]$ controls skew intensity.

When the vault is long ($\mathtt{skew} > 0$), bid size decreases and ask size increases — the vault offers more on the sell side to unwind. Total quoted size ($\mathtt{bidSize} + \mathtt{askSize} = 2 \times \mathtt{baseSize}$) is preserved.

### Bid price

$$
\mathtt{rawBid} = \mathtt{oraclePrice} \times \bigl(1 - \mathtt{halfSpread} \times (1 + \mathtt{skew} \times \mathtt{spreadSkewFactor})\bigr)
$$

Snap **down** to the nearest tick:

$$
\mathtt{bidPrice} = \mathtt{rawBid} - (\mathtt{rawBid} \bmod \mathtt{tickSize})
$$

**Book-crossing prevention:** if $\mathtt{bidPrice} \geq \mathtt{bestAsk}$, clamp to $\mathtt{bestAsk} - \mathtt{tickSize}$.

Skip if $\mathtt{bidPrice} \leq 0$ or notional is below the minimum order size.

When the vault is long, the bid spread widens (less likely to accumulate more).

### Ask price

$$
\mathtt{rawAsk} = \mathtt{oraclePrice} \times \bigl(1 + \mathtt{halfSpread} \times (1 - \mathtt{skew} \times \mathtt{spreadSkewFactor})\bigr)
$$

Snap **up** to the nearest tick (ceiling):

$$
\mathtt{askPrice} = \begin{cases}
\mathtt{rawAsk} & \text{if } \mathtt{rawAsk} \bmod \mathtt{tickSize} = 0 \\
\mathtt{rawAsk} - (\mathtt{rawAsk} \bmod \mathtt{tickSize}) + \mathtt{tickSize} & \text{otherwise}
\end{cases}
$$

**Book-crossing prevention:** if $\mathtt{askPrice} \leq \mathtt{bestBid}$, clamp to $\mathtt{bestBid} + \mathtt{tickSize}$.

Skip if notional is below the minimum order size.

When the vault is long, the ask spread tightens (more attractive to takers who buy from the vault).

### Combined effect

When the vault is long, all four levers push toward unwinding:

1. Bid size decreases (less buying)
2. Ask size increases (more selling)
3. Bid spread widens (buys less likely to fill)
4. Ask spread tightens (sells more likely to fill)

The mirror applies when short.

### Per-pair parameters

| Parameter                  | Role                                         |
| -------------------------- | -------------------------------------------- |
| `initial_margin_ratio`     | Used to compute margin-constrained size      |
| `min_order_size`           | Minimum notional to place an order           |
| `tick_size`                | Price granularity for snapping               |
| `vault_half_spread`        | Base half bid-ask spread around oracle price |
| `vault_liquidity_weight`   | Weight for margin allocation across pairs    |
| `vault_max_quote_size`     | Maximum base size per side                   |
| `vault_max_skew_size`      | Position size at which skew saturates        |
| `vault_size_skew_factor`   | Size skew intensity ($[0, 1]$)               |
| `vault_spread_skew_factor` | Spread skew intensity ($\ge 0$)              |

If any of `vault_half_spread`, `vault_max_quote_size`, `vault_liquidity_weight`, `tick_size`, or the allocated margin is zero, the vault skips quoting for that pair.

### Choosing parameters

**`vault_max_skew_size`** — the position size at which skew reaches its maximum. A natural starting point is `vault_max_quote_size` (the existing per-side cap). This means: once the vault has accumulated one full order's worth of directional exposure, skew is fully engaged. For gentler unwinding, use `2x vault_max_quote_size`.

**`vault_size_skew_factor`** — how aggressively to tilt order sizes. Start with **0.5**: at maximum skew, the heavier side quotes 1.5x and the lighter side 0.5x. A value of 1.0 fully shuts off quoting on one side at max position, which may be too aggressive for a vault that should always provide some liquidity. Range **0.5 to 0.8** is recommended.

**`vault_spread_skew_factor`** — how aggressively to tilt spreads. Start with **0.3**: at maximum skew, the tightened side has 70% of normal spread and the widened side has 130%. Keep this below `vault_size_skew_factor` — size adjustment is the primary lever, spread adjustment is the fine-tuning. Range **0.3 to 0.5** is recommended. Values above 1.0 are permitted and cause the tightened side to cross the oracle price at maximum skew (an aggressive-unwind posture, useful for quickly deleveraging a large directional position); the invariant `bid < ask` still holds since `ask - bid = 2 × oracle × vault_half_spread`. The effective upper bound is governed by the cross-field invariant `vault_half_spread × (1 + vault_spread_skew_factor) < 1`, which ensures the bid stays positive at max skew.

**General tuning principle:** start conservative (size 0.5, spread 0.3), observe PnL and position behavior, increase if the vault still accumulates too much directional exposure.
