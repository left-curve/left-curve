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

The vault uses its margin to market make in the order book. For now, it does so following a naïve policy. We expect to optimize this in the future.

Each block, after the oracle update, the vault cancels all existing quotes and recomputes bid/ask orders for every pair.

### Margin allocation

Total vault **available margin** is split across pairs by weight:

$$
\mathtt{pairMargin} = \mathtt{vaultAvailableMargin} \times \frac{\mathtt{vaultLiquidityWeight}}{\mathtt{vaultTotalWeight}}
$$

where $\mathtt{vaultAvailableMargin} = \max(0,\; \mathtt{equity} - \mathtt{usedMargin})$ and $\mathtt{usedMargin}$ is the sum of initial margin across all vault positions (see [Margin §8](1-margin.md#8-available-margin)).

### Quote size

Each side receives half the allocated margin, capped by a per-pair maximum:

$$
\mathtt{halfMargin} = \frac{\mathtt{pairMargin}}{2}
$$

$$
\mathtt{quoteSize} = \min \left(\frac{\mathtt{halfMargin}}{\mathtt{oraclePrice} \times \mathtt{imr}},\; \mathtt{maxQuoteSize}\right)
$$

where $\mathtt{imr}$ is the initial margin ratio.

### Bid price

$$
\mathtt{rawBid} = \mathtt{oraclePrice} \times (1 - \mathtt{halfSpread})
$$

Snap **down** to the nearest tick:

$$
\mathtt{bidPrice} = \mathtt{rawBid} - (\mathtt{rawBid} \bmod \mathtt{tickSize})
$$

**Book-crossing prevention:** if $\mathtt{bidPrice} \geq \mathtt{bestAsk}$, clamp to $\mathtt{bestAsk} - \mathtt{tickSize}$.

Skip if $\mathtt{bidPrice} \leq 0$ or notional is below the minimum order size.

### Ask price

$$
\mathtt{rawAsk} = \mathtt{oraclePrice} \times (1 + \mathtt{halfSpread})
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

### Per-pair parameters

| Parameter                | Role                                        |
| ------------------------ | ------------------------------------------- |
| `vault_half_spread`      | Half the bid-ask spread around oracle price |
| `vault_max_quote_size`   | Maximum size per side                       |
| `vault_liquidity_weight` | Weight for margin allocation across pairs   |
| `tick_size`              | Price granularity for snapping              |
| `initial_margin_ratio`   | Used to compute margin-constrained size     |
| `min_position_size`      | Minimum notional to place a quote           |

If any of `vault_half_spread`, `vault_max_quote_size`, `vault_liquidity_weight`, `tick_size`, or the allocated margin is zero, the vault skips quoting for that pair.
