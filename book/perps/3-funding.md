# Funding

Fundings are periodic payments between longs and shorts that anchor the perpetual contract price to the oracle. When the market trades above the oracle, longs pay shorts; when below, shorts pay longs. This mechanism discourages persistent deviations from the spot price without requiring contract expiry.

## 1. Premium

Each funding cycle begins with measuring how far the on-chain book has drifted from the oracle. The contract computes two **impact prices** by walking the book, takes their midpoint, and compares it to the oracle:

- **Impact bid** — the volume-weighted average price (VWAP) obtained by selling $\mathtt{impactSize}$ worth of base asset into the bid side.
- **Impact ask** — the VWAP obtained by buying $\mathtt{impactSize}$ worth from the ask side.

The premium is then:

$$
\mathtt{midImpactPrice} = \frac{\mathtt{impactBid} + \mathtt{impactAsk}}{2}
$$

$$
\mathtt{premium} = \frac{\mathtt{midImpactPrice} - \mathtt{oracle}}{\mathtt{oracle}}
$$

If a side of the book has less than $\mathtt{impactSize}$ of depth, the walk returns the VWAP of whatever depth is available. If a side has no depth at all, the sample is skipped for that cycle rather than a one-sided mid being computed. In steady state both sides are always populated by the vault.

## 2. Sampling

A cron job runs frequently (e.g. every minute). Each invocation samples the premium for every active pair and accumulates it into the pair's state:

$$
\mathtt{premiumSum} \mathrel{+}= \mathtt{premium}
$$

$$
\mathtt{premiumSamples} \mathrel{+}= 1
$$

Sampling at a cadence close to the block rate gives each observation roughly equal weight. A resting order that momentarily drags the mid can only influence the average in proportion to how long it sits on the book relative to the full funding period.

## 3. Collection

When $\mathtt{fundingPeriod}$ has elapsed since the last collection, the same cron invocation finalises the funding rate:

1. **Average premium:**

   $$
   \mathtt{avgPremium} = \mathtt{premiumSum} \mathbin{/} \mathtt{premiumSamples}
   $$

2. **Clamp** to the configured bounds:

   $$
   \mathtt{rate} = \mathrm{clamp} \bigl( \mathtt{avgPremium},\; [-\mathtt{maxAbsFundingRate},\; +\mathtt{maxAbsFundingRate}] \bigr)
   $$

3. **Funding delta** — scale by the actual elapsed interval and oracle price:

   $$
   \mathtt{fundingDelta} = \mathtt{rate} \times \mathtt{interval} \times \mathtt{oraclePrice}
   $$

4. **Accumulate** into the pair-level running total:

   $$
   \mathtt{fundingPerUnit} \mathrel{+}= \mathtt{fundingDelta}
   $$

5. **Reset** accumulators: $\mathtt{premiumSum} \gets 0$, $\mathtt{premiumSamples} \gets 0$, $\mathtt{lastFundingTime} \gets \mathtt{now}$.

## 4. Position-level settlement

Accrued funding is settled on a position whenever it is touched — during a fill, liquidation, or ADL event:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

After settlement the entry point is reset:

$$
\mathtt{entryFundingPerUnit} \gets \mathtt{fundingPerUnit}
$$

**Sign convention:** positive accrued funding is a cost to the holder (longs pay when the rate is positive, shorts pay when it is negative). The negated accrued funding is added to the user's realised PnL. See [Order matching §7a](2-order-matching.md#7a-funding-settlement) and [Vault §4](5-vault.md) for how this integrates with fill execution and vault accounting.

## 5. Parameters

| Field                      | Type            | Description                                                                                                                                                                                                     |
| -------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `funding_period`           | `Duration`      | Minimum time between funding collections.                                                                                                                                                                       |
| `impact_size`              | `UsdValue`      | Notional depth walked on each side of the book to compute impact prices. A larger value dilutes the influence of any single resting order on the premium in proportion to the fraction of the walk it occupies. |
| `max_abs_funding_rate`     | `FundingRate`   | Symmetric clamp applied to the average premium before scaling to a delta. Prevents runaway rates during prolonged skew.                                                                                         |
| `funding_rate_multiplier`  | `Dimensionless` | Scalar applied to the vault-driven premium so governance can tune funding independently of the vault's quoting (see §6). Bounds: $\geq 0$. $1$ is identity; $0$ disables funding for the pair.                  |

## 6. Discussions

### Vault being the sole maker

As of today, the [protocol-owned vault](5-vault.md) is the dominant maker in Dango's markets. The [vault's inventory-skew-aware quoting policy](5-vault.md#5-market-making-policy) causes the book mid to drift from the oracle whenever the vault holds inventory:

$$
\mathtt{vaultBid} = \mathtt{oracle} \cdot (1 - \mathtt{halfSpread} \cdot (1 + \mathtt{skew} \cdot \mathtt{spreadSkewFactor}))
$$

$$
\mathtt{vaultAsk} = \mathtt{oracle} \cdot (1 + \mathtt{halfSpread} \cdot (1 - \mathtt{skew} \cdot \mathtt{spreadSkewFactor}))
$$

Suppose the vault is literally the only maker in the entire market, we can substitute the vault's bid and ask into the $\mathtt{midImpactPrice}$ formula:

$$
\mathtt{midImpactPrice} = \mathtt{oracle} \cdot (1 - \mathtt{halfSpread} \cdot \mathtt{skew} \cdot \mathtt{spreadSkewFactor})
$$

and therefore

$$
\mathtt{premium} = -\mathtt{halfSpread} \cdot \mathtt{skew} \cdot \mathtt{spreadSkewFactor} \cdot \mathtt{fundingRateMultiplier}
$$

Positive skew (vault long, because sell flow has dominated) produces a negative premium, so longs receive funding from shorts — which credits the vault-as-long for absorbed inventory. Symmetric when short. The sign is economically correct by construction.

The closed-form also tethers funding to the vault's quoting parameters: tightening spreads to compete for flow would otherwise shrink funding by the same factor. $\mathtt{fundingRateMultiplier}$ is a per-pair governance knob that decouples these two — admins can dial funding up or down (e.g. in response to persistent one-sided skew) without touching $\mathtt{halfSpread}$ or $\mathtt{spreadSkewFactor}$ and therefore without changing the vault's quoted prices. $\mathtt{fundingRateMultiplier} = 1$ is the identity and matches the pre-multiplier formulation; $0$ disables funding entirely.

### Comparison with other exchanges

The "book mid minus oracle" premium is the dominant on-chain perpetual-funding pattern — see [Drift](https://docs.drift.trade/protocol/trading/perpetuals-trading/funding-rates) (bid/ask TWAP mid vs oracle TWAP), [Vertex](https://vertex-protocol.gitbook.io/docs/basics/funding-rates) (mark vs spot index), [Paradex](https://docs.paradex.trade/risk/funding-mechanism) (Fair Basis from mark), and [MCDEX v2](https://mcdex.medium.com/introduce-mcdex-v2-perpetual-c97b18ff4e23) (AMM mid vs index). Dango's formulation differs in reading impact prices (depth-walked VWAPs) rather than top-of-book, which bakes depth distribution into the primitive and forces any book-level manipulation to commit notional proportional to $\mathtt{impactSize}$.
