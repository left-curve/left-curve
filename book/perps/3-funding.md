# Funding

Fundings are periodic payments between longs and shorts that anchor the
perpetual contract price to the oracle. When the market trades above the oracle,
longs pay shorts; when below, shorts pay longs. This mechanism discourages
persistent deviations from the spot price without requiring contract expiry.

## 1 Premium

Each funding cycle begins with measuring how far the order book deviates from
the oracle. The contract computes two **impact prices** by walking the book:

- **Impact bid** — the volume-weighted average price (VWAP) obtained by selling
  $\mathtt{impactSize}$ worth of base asset into the bid side.
- **Impact ask** — the VWAP obtained by buying $\mathtt{impactSize}$ worth
  from the ask side.

The premium is then:

$$
\mathtt{premium} = \frac{\max(0,\;\mathtt{impactBid} - \mathtt{oracle}) - \max(0,\;\mathtt{oracle} - \mathtt{impactAsk})}{\mathtt{oracle}}
$$

If either side has insufficient depth to fill $\mathtt{impactSize}$, its
$\max(0, \ldots)$ term contributes zero. When both sides lack depth, the premium
is zero.

## 2 Sampling

A cron job runs frequently (e.g. every minute). Each invocation samples the
premium for every active pair and accumulates it into the pair's state:

$$
\mathtt{premiumSum} \mathrel{+}= \mathtt{premium}
$$

$$
\mathtt{premiumSamples} \mathrel{+}= 1
$$

Sampling more frequently than collecting gives the average premium resilience
against momentary spikes — a single large order cannot dominate the rate.

## 3 Collection

When $\mathtt{fundingPeriod}$ has elapsed since the last collection, the same
cron invocation finalises the funding rate:

1. **Average premium:**

   $$
   \mathtt{avgPremium} = \mathtt{premiumSum} \mathbin{/} \mathtt{premiumSamples}
   $$

2. **Clamp** to the configured bounds:

   $$
   \mathtt{rate} = \mathrm{clamp}\!\bigl(\mathtt{avgPremium},\; [-\mathtt{maxAbsFundingRate},\; +\mathtt{maxAbsFundingRate}]\bigr)
   $$

3. **Funding delta** — scale by the actual elapsed interval and oracle price:

   $$
   \mathtt{fundingDelta} = \mathtt{rate} \times \mathtt{interval} \times \mathtt{oraclePrice}
   $$

4. **Accumulate** into the pair-level running total:

   $$
   \mathtt{fundingPerUnit} \mathrel{+}= \mathtt{fundingDelta}
   $$

5. **Reset** accumulators: $\mathtt{premiumSum} \gets 0$,
   $\mathtt{premiumSamples} \gets 0$, $\mathtt{lastFundingTime} \gets \mathtt{now}$.

## 4 Position-level settlement

Accrued funding is settled on a position whenever it is touched — during a fill,
liquidation, or ADL event:

$$
\mathtt{accruedFunding} = \mathtt{size} \times (\mathtt{fundingPerUnit} - \mathtt{entryFundingPerUnit})
$$

After settlement the entry point is reset:

$$
\mathtt{entryFundingPerUnit} \gets \mathtt{fundingPerUnit}
$$

**Sign convention:** positive accrued funding is a cost to the holder (longs pay
when the rate is positive, shorts pay when it is negative). The negated accrued
funding is added to the user's realised PnL. See
[Order matching §6a](2-order-matching.md#6a-funding-settlement) and
[Vault §4](4-vault.md) for how this integrates with fill execution and vault
accounting.

## 5 Parameters

| Field                  | Type          | Description                                                                                                             |
| ---------------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `funding_period`       | `Duration`    | Minimum time between funding collections.                                                                               |
| `impact_size`          | `UsdValue`    | Notional depth walked on each side of the book to compute impact prices.                                                |
| `max_abs_funding_rate` | `FundingRate` | Symmetric clamp applied to the average premium before scaling to a delta. Prevents runaway rates during prolonged skew. |
