# Funding

Fundings are periodic payments between longs and shorts that anchor the perpetual contract price to the oracle. When the market trades above the oracle, longs pay shorts; when below, shorts pay longs. This mechanism discourages persistent deviations from the spot price without requiring contract expiry.

## 1. Overview

Dango implements **two funding mechanisms** that differ in how they compute the premium — the measure of how far the perpetual market has drifted from the oracle:

- **Impact-price model** — derives the premium from the state of the order book. This is a _maker-side_ signal: it reflects where liquidity providers are willing to quote.
- **VWAP (volume-weighed average price) model** — derives the premium from a rolling volume-weighted average of recent taker fills. In addition to the maker-side price signal (each fill's price is set by a resting maker order), it also captures a _taker-side_ volume signal: which side of the book is being hit, and how often.

A particularity of Dango at this stage of its lifecycle is that [the protocol-owned counterparty vault](5-vault.md) is effectively the sole maker in the market, as the exchange was newly released. In such a sole-maker setting, the impact-price model suffers from two flaws:

- **Lagging** — the general flaw in any sole-maker market. In a competitive market, makers reprice aggressively in response to flow, so the book-derived premium moves as flow arrives. With only one maker, the book instead reflects only the vault's reactive adjustments to its own accumulated inventory (via the inventory skew policy described in [Vault §5](5-vault.md#5-market-making-policy)). Since inventory is itself a consequence of past flow, the book-derived signal always trails the flow that produced it — the vault must first absorb directional exposure before the signal appears.
- **Structurally zero** — a stricter failure specific to Dango's impact-price formula combined with the vault's quoting policy. The formula (defined in [§2](#2-impact-price-model)) yields a non-zero premium only when the book has _crossed_ the oracle — that is, when the mismatch between the book and the oracle is severe enough that even the impact bid sits _above_ the oracle, or, symmetrically, the impact ask sits _below_ it. In a competitive market, such crossings occur during strong directional flow or when the oracle lags a rapid price move; the formula is effectively a spike detector for those episodes. But the vault's quoting policy places bids strictly below the oracle and asks strictly above, and its inventory-skew adjustment is mathematically bounded such that the skew-adjusted quotes never cross oracle under any inventory state. With the vault being the only source of book depth, the impact prices always sit on their "natural" sides of the oracle, and the premium is zero regardless of actual taker flow or vault inventory.

The VWAP model, in contrast, avoids both flaws. It is a **leading indicator** that reflects the pressure in taker flow directly, _before_ that flow has pushed the vault into a skewed inventory, and its linear weighted-average formula responds to any asymmetry in fill prices without requiring the book to cross the oracle.

For this reason, at this time, although both mechanisms are implemented in the contract, **only the VWAP model is active**. Once the market matures to include multiple competitive makers — at which point book-based signals become reliable and harder to manipulate — we expect to shift to a hybrid model that combines both signals.

The sections [§2](2-impact-price-model) and [§3](3-vwap-model) describe each mechanism in turn. Downstream collection ([§4](#4-collection)), settlement ([§5](#5-position-level-settlement)), and parameter ([§6](#6-parameters)) machinery is shared: both mechanisms produce an average premium that feeds into the same rate-calculation pipeline.

## 2. Impact-price model

Each funding cycle begins with measuring how far the order book deviates from the oracle. The contract computes two **impact prices** by walking the book:

- **Impact bid** — the volume-weighted average price (VWAP) obtained by selling $\mathtt{impactSize}$ worth of base asset into the bid side.
- **Impact ask** — the VWAP obtained by buying $\mathtt{impactSize}$ worth from the ask side.

The premium is then:

$$
\mathtt{premium} = \frac{\max(0,\;\mathtt{impactBid} - \mathtt{oracle}) - \max(0,\;\mathtt{oracle} - \mathtt{impactAsk})}{\mathtt{oracle}}
$$

If either side has insufficient depth to fill $\mathtt{impactSize}$, its $\max(0, \ldots)$ term contributes zero. When both sides lack depth, the premium is zero.

### Sampling

A cron job runs frequently (e.g. every minute). Each invocation samples the premium for every active pair and accumulates it into the pair's state:

$$
\mathtt{premiumSum} \mathrel{+}= \mathtt{premium}
$$

$$
\mathtt{premiumSamples} \mathrel{+}= 1
$$

Sampling more frequently than collecting gives the average premium resilience against momentary spikes — a single large order cannot dominate the rate.

At collection time, the average premium is computed as:

$$
\mathtt{avgPremium} = \mathtt{premiumSum} \mathbin{/} \mathtt{premiumSamples}
$$

## 3. VWAP model

The VWAP model computes the premium as a volume-weighted average of recent taker fills against the oracle. For each taker fill produced by `submit_order` (liquidation-driven fills are excluded, to avoid contaminating the signal with forced unwinds), the contract records:

- $\mathtt{oraclePrice}$ — the oracle price for the pair at the block of the fill
- $\mathtt{fillPrice}$ — the price at which the fill executed
- $\mathtt{fillSize}$ — the fill size in the base asset (positive for bids, negative for asks)

and updates two per-pair running sums:

$$
\mathtt{deltaSum} \mathrel{+}= (\mathtt{fillPrice} - \mathtt{oraclePrice}) \times |\mathtt{fillSize}|
$$

$$
\mathtt{oracleNotionalSum} \mathrel{+}= |\mathtt{oraclePrice} \times \mathtt{fillSize}|
$$

A fill at a price _above_ the oracle contributes positively to $\mathtt{deltaSum}$; a fill _below_ contributes negatively. Because taker buys hit asks (which, under normal market conditions, sit above the oracle) and taker sells hit bids (below the oracle), the sign of the running $\mathtt{deltaSum}$ naturally tracks the direction of net taker pressure — positive when buying dominates, negative when selling dominates. No separate "taker direction" flag is needed in the formula; the sign of $(\mathtt{fillPrice} - \mathtt{oraclePrice})$ is itself a proxy for which side of the book was hit.

### Bucketing

Rather than store one state row per fill, fills are aggregated into fixed-width time buckets of duration $\mathtt{vwapBucketSize}$. Each bucket stores the per-bucket sums $(\mathtt{deltaSum}, \mathtt{oracleNotionalSum})$ over all fills whose block time falls within its interval. This bounds on-chain state growth: the number of buckets retained per pair is at most $\mathtt{vwapWindow} / \mathtt{vwapBucketSize}$ plus a small grace margin for pruning.

### Windowed premium

At collection time, the contract iterates all buckets whose start falls within the last $\mathtt{vwapWindow}$, sums their components, and computes:

$$
\mathtt{avgPremium} = \frac{\sum \mathtt{deltaSum}}{\sum \mathtt{oracleNotionalSum}}
$$

where the sums run over all buckets in the window.

This is the **oracle-notional-weighted average of each fill's per-trade premium** $(\mathtt{fillPrice} - \mathtt{oraclePrice}) / \mathtt{oraclePrice}$, with each fill's comparison anchored to the oracle at its own moment — so mid-window oracle drift does not distort the signal.

If the window contains no trades (denominator is zero), the premium is zero.

### Pruning

At the end of each collection cycle, buckets older than twice the VWAP window are removed from storage. Retaining two full windows (rather than one) provides a grace margin in case a collection runs later than expected and needs to look back past the nominal cutoff.

## 4. Collection

When $\mathtt{fundingPeriod}$ has elapsed since the last collection, the cron invocation finalises the funding rate. The procedure is identical regardless of which model produced $\mathtt{avgPremium}$:

1. **Clamp** to the configured bounds:

   $$
   \mathtt{rate} = \mathrm{clamp} \bigl( \mathtt{avgPremium},\; [-\mathtt{maxAbsFundingRate},\; +\mathtt{maxAbsFundingRate}] \bigr)
   $$

2. **Funding delta** — scale by the actual elapsed interval and oracle price:

   $$
   \mathtt{fundingDelta} = \mathtt{rate} \times \mathtt{interval} \times \mathtt{oraclePrice}
   $$

3. **Accumulate** into the pair-level running total:

   $$
   \mathtt{fundingPerUnit} \mathrel{+}= \mathtt{fundingDelta}
   $$

4. **Reset** state: $\mathtt{lastFundingTime} \gets \mathtt{now}$. For the impact-price model, $\mathtt{premiumSum} \gets 0$ and $\mathtt{premiumSamples} \gets 0$. For the VWAP model, expired buckets are pruned as described above.

## 5. Position-level settlement

Accrued funding is settled on a position whenever it is touched — during a fill, liquidation, or ADL event:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

After settlement the entry point is reset:

$$
\mathtt{entryFundingPerUnit} \gets \mathtt{fundingPerUnit}
$$

**Sign convention:** positive accrued funding is a cost to the holder (longs pay when the rate is positive, shorts pay when it is negative). The negated accrued funding is added to the user's realised PnL. See [Order matching §7a](2-order-matching.md#7a-funding-settlement) and [Vault §4](5-vault.md) for how this integrates with fill execution and vault accounting.

## 6. Parameters

| Field                  | Type          | Description                                                                                                             |
| ---------------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `funding_period`       | `Duration`    | Minimum time between funding collections.                                                                               |
| `impact_size`          | `UsdValue`    | Notional depth walked on each side of the book to compute impact prices. Impact-price model only.                       |
| `vwap_window`          | `Duration`    | Rolling window over which taker fills are volume-weighted into a premium. VWAP model only.                              |
| `vwap_bucket_size`     | `Duration`    | Granularity at which taker fills are aggregated into state rows. Must evenly divide `vwap_window`. VWAP model only.     |
| `max_abs_funding_rate` | `FundingRate` | Symmetric clamp applied to the average premium before scaling to a delta. Prevents runaway rates during prolonged skew. |
