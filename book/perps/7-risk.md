# Risk Parameters

This chapter describes how to choose the risk parameters that govern the perpetual futures exchange — the global `Param` fields and per-pair `PairParam` fields defined in the perps contract. The goal is a systematic, reproducible calibration workflow that balances capital efficiency against tail-risk protection.

## 1. Margin ratios

The **initial margin ratio** (IMR) sets maximum leverage ($1 / \mathtt{imr}$). The **maintenance margin ratio** (MMR) sets the liquidation threshold. Both are per-pair.

### 1.1 Volatility-based derivation

Start from the asset's historical daily return distribution:

1. Collect at least 1 year of daily log-returns.
2. Compute the 99.5th-percentile absolute daily return $r_{99.5}$.
3. Apply a **liquidation-delay factor** $k$ (typically 2–3) to account for the time between the price move and the liquidation execution:

   $$
   \mathtt{mmr} = r_{99.5} \times k
   $$

4. Set IMR as a multiple of MMR:

   $$
   \mathtt{imr} = \mathtt{mmr} \times m, \quad m \in [1.5,\; 4]
   $$

A higher $m$ gives more buffer between position entry and liquidation, reducing bad-debt risk at the cost of lower leverage.

### 1.2 Peer benchmarks

| Asset | Hyperliquid max leverage | Hyperliquid IMR | dYdX IMR |
| ----- | ------------------------ | --------------- | -------- |
| BTC   | 40×                      | 2.5 %           | 5 %      |
| ETH   | 25×                      | 4 %             | 5 %      |
| SOL   | 20×                      | 5 %             | 10 %     |
| HYPE  | 10×                      | 10 %            | —        |

### 1.3 Invariants

The following must hold for every pair:

$$
0 < \mathtt{mmr} < \mathtt{imr} \leq 1
$$

$$
\mathtt{liquidationFeeRate} \leq \mathtt{mmr} - \mathrm{takerFeeRate}
$$

The second constraint ensures a liquidated position can always cover the taker fee and liquidation fee from the maintenance margin cushion.

## 2. Fee rates

Three fee rates apply globally (not per-pair):

| Parameter              | Role                                                       |
| ---------------------- | ---------------------------------------------------------- |
| `maker_fee_rate`       | Charged on limit-order fills; revenue to the vault         |
| `taker_fee_rate`       | Charged on market / crossing fills; revenue to the vault   |
| `liquidation_fee_rate` | Charged on liquidation notional; revenue to insurance fund |

### 2.1 Sizing principles

- **Taker fee** should exceed the typical half-spread of the most liquid pair so the vault earns positive expected value on every fill against a taker.
- **Maker fee** can be zero, slightly positive, or negative (rebate). A zero maker fee attracts resting liquidity; a negative maker fee pays the maker on every fill. The absolute value of the maker fee rate must not exceed the taker fee rate, otherwise the exchange loses money on each trade.
- **Liquidation fee** must satisfy the invariant in [§1.3](#13-invariants). It should be large enough to fund the insurance pool but small enough that a liquidated user retains some margin when possible.

### 2.2 Industry benchmarks

| Exchange    | Maker  | Taker  |
| ----------- | ------ | ------ |
| Hyperliquid | 0.015% | 0.045% |
| dYdX        | 0.01%  | 0.05%  |
| GMX         | 0.05%  | 0.07%  |

## 3. Funding parameters

Funding anchors the perp price to the oracle. Two per-pair parameters and one global parameter control its behaviour (see [Funding](3-funding.md) for mechanics):

| Parameter              | Scope    | Calibration guidance                                                             |
| ---------------------- | -------- | -------------------------------------------------------------------------------- |
| `funding_period`       | Global   | 1–8 hours. Shorter periods track the premium more tightly but increase gas cost. |
| `max_abs_funding_rate` | Per-pair | See [§3.1](#31-max-funding-rate).                                                |
| `impact_size`          | Per-pair | See [§3.2](#32-impact-size).                                                     |

### 3.1 Max funding rate

The max daily funding rate limits how much a position can be charged per day. A useful rule of thumb:

$$
\mathtt{maxAbsFundingRate_{\mathrm{daily}}} \leq \frac{\mathtt{imr}}{T_{\mathrm{liq}}}
$$

where $T_{\mathrm{liq}}$ is the number of days it should take sustained max-rate funding to liquidate a fully leveraged position. For $T_{\mathrm{liq}} = 3$ days and $\mathtt{imr} = 5\%$:

$$
\mathtt{maxRate} = \frac{0.05}{3} \approx 1.67\% \text{ per day}
$$

### 3.2 Impact size

The `impact_size` determines how deep the order book is walked to compute the premium. Set it to a representative trade size — large enough that the premium reflects real depth, small enough that thin books don't produce zero premiums too often. A good starting point is 1–5% of the target max OI.

## 4. Capacity parameters

### 4.1 Max open interest

The maximum OI per side caps the exchange's aggregate exposure:

$$
\mathtt{maxAbsOI} \leq \frac{\mathtt{vaultEquity} \times w_{\mathrm{pair}}}{\mathtt{mmr} \times \mathtt{tailLossFactor}}
$$

where $w_{\mathrm{pair}}$ is the pair's weight fraction and $\mathtt{tailLossFactor}$ (2–5) is a safety multiplier reflecting how many times maintenance margin the vault could lose in a tail event.

Start conservatively — it is easy to raise OI caps but dangerous to lower them (existing positions above the cap cannot be force-closed).

### 4.2 Lot size

The **lot size** is the precision constraint on an order's `size` field: every submitted order must satisfy

$$
\mathtt{size} \bmod \mathtt{lotSize} = 0
$$

and the resulting positions any trader can hold are therefore always integer multiples of `lot_size`. The smallest non-zero position in the system is exactly one lot.

This is the protocol's **primary dust-prevention mechanism**. By making the position-size space discrete (rather than continuous down to `Quantity` precision, currently $10^{-6}$ of the asset), residual positions after partial fills, liquidations, ADL, or close orders cannot shrink below a known floor.

#### Why discrete lots prevent dust

A lot-size constraint sidesteps an entire class of dust-formation paths that arise in continuous-precision systems:

- **Order submission.** Orders with `size` not divisible by `lot_size` are rejected outright. Users cannot accidentally place a 0.0001234 ETH order; it must be rounded to a lot boundary.
- **Partial fills.** When two lot-aligned orders match, the fill size is `min(taker_remaining, maker_resting)`. Both operands are lot-aligned, so the fill is too. Multiple fills against the same maker order remain lot-aligned by induction.
- **Position arithmetic.** A position is the sum of a finite sequence of fills, each lot-aligned. The sum is lot-aligned. No path through the matching engine can produce a sub-lot residual.
- **Liquidations and ADL.** Close amounts are clamped to the user's existing lot-aligned position size. Whatever the liquidation engine decides to close, the result is lot-aligned. There is no need for snap-to-zero or snap-to-min logic at fill time.
- **Triggered conditional (TP/SL) orders.** The trigger size, if specified explicitly, must already be lot-aligned at submission. Clamping at trigger time preserves lot alignment.

The resulting invariant — *every position is a non-negative multiple of `lot_size`* — is straightforward to reason about and requires no oracle-price-dependent calculation, no future-state prediction, and no special cases inside the matching engine.

#### What lot size does not do

Lot size bounds *precision*, not *USD value*. At extreme oracle prices, one lot can still be worth pennies. For example, `lot_size = 0.0001 BTC` is worth $5 at BTC = \$50{,}000$ but only $0.10 at a hypothetical BTC = \$1{,}000$. The protocol accepts that:

- Per-position storage and indexing cost is constant in the position size — a 1-lot position costs the same to track as a 1000-lot one.
- Funding accrual on a $0.10 notional position is computationally and economically negligible.
- A user holding a 1-lot dust position can always full-close it (`size = -current_size`), since lot alignment is preserved by definition for a full close.

A complementary **min liquidation value** parameter — planned for a follow-up upgrade — will handle the orthogonal concern of "is this position economically worth liquidating given gas costs?" That parameter targets the value-not-precision dimension and operates at liquidation time rather than at order submission.

#### Industry alignment

Discrete lot sizes are the primary dust-prevention mechanism on every major orderbook-based perpetual exchange:

| Exchange         | Mechanism                                                  | Notional backstop  |
| ---------------- | ---------------------------------------------------------- | ------------------ |
| Hyperliquid      | Per-pair decimal precision (BTC: 4 dp, ETH: 3 dp, ...)     | `MinTradeNtl` $10  |
| Binance Futures  | Per-contract step size (BTCUSDT: 0.001 BTC, ...)           | $5 minimum order   |
| dYdX v4          | `step_base_quantums` per market                            | $1 minimum order   |

#### Choosing lot sizes

A reasonable starting point is a `lot_size` such that one lot at typical oracle prices falls in the **\$1–\$10** notional range:

| Asset (typical price) | Suggested `lot_size` | One lot at typical price |
| --------------------- | -------------------- | ------------------------ |
| BTC (\$80k)           | $10^{-4}$ BTC        | \$8                      |
| ETH (\$3k)            | $10^{-3}$ ETH        | \$3                      |
| SOL (\$90)            | $10^{-1}$ SOL        | \$9                      |
| HYPE (\$50)           | $10^{-1}$ HYPE       | \$5                      |

Smaller lots improve UX (finer-grained position control) at the cost of allowing more dust-like positions when the oracle drops. Larger lots simplify the price-display rounding for retail users but exclude small-account traders. Aligning lot sizes with peer exchanges is recommended so users can size positions consistently across venues.

The chosen value should be **at least one `Quantity` ULP** ($10^{-6}$ of the asset) and ideally several orders of magnitude larger so the precision constraint is meaningful.

`lot_size` is paired with [`min_order_value` (§4.3)](#43-min-order-value), which adds a value-dimension floor on top of the precision-dimension floor enforced here.

### 4.3 Min order value

Where [`lot_size` (§4.2)](#42-lot-size) enforces a *precision* floor on order quantity, `min_order_value` enforces a **value floor** on order notional. A submitted order's notional must clear the threshold:

$$
|\mathtt{size}| \times \mathtt{oraclePrice} \geq \mathtt{minOrderValue}
$$

Typical values are \$5–\$10, matching Hyperliquid's `MinTradeNtl` (\$10) and Binance Futures' \$5 minimum. The two checks compose: a submitted order must satisfy both `lot_size` and `min_order_value`. Lot size handles the precision dimension; `min_order_value` handles the value dimension. Neither subsumes the other.

#### Why a value floor exists alongside lot size

Lot size guarantees that no position is precision-dust — every position is at least one lot. But "one lot" is denominated in the asset; its USD notional fluctuates with the oracle. At extreme price drops, one lot can be worth pennies — too small to recover the gas cost of trading or maintaining it.

`min_order_value` is the secondary defense for that case. It prevents users from *submitting* tiny-notional orders even when those orders would be lot-aligned. Two concrete classes of pathology it blocks:

- **Gas-spam.** Validating, matching, and event-emitting an order has a cost roughly independent of its notional. Without `min_order_value`, a user could submit thousands of pennies-each lot-aligned orders for the same gas cost as a single meaningful order.
- **Funding-accrual cost.** Each open position triggers per-block funding work. A position worth pennies pays funding measured in fractions of a cent per period; the gas to compute the funding exceeds the funding payment by orders of magnitude.

#### The close-always-allowed exemption

Applying `min_order_value` uniformly would create a **dust trap**: a user holding a sub-min position (1-lot dust at a low oracle price, or a position acquired before a price drop) could not submit any close order, because the close order's own notional would itself be below the threshold. The user's only escape would be to wait for the oracle to recover.

To prevent that, the protocol exempts **pure reductions** from the `min_order_value` check. A pure reduction is an order whose decomposition

$$
(\mathtt{closingSize}, \mathtt{openingSize}) = \mathtt{decomposeFill}(\mathtt{size}, \mathtt{currentPosition})
$$

— after reduce-only truncation, which forces $\mathtt{openingSize} = 0$ — satisfies $\mathtt{openingSize} = 0$ and $\mathtt{closingSize} \neq 0$.

In words: an order that only *reduces* existing exposure is allowed regardless of notional; an order that adds new exposure (opens from flat, grows an existing position, or flips direction) must clear `min_order_value` on the full order notional.

##### When the exemption applies

The decomposition above is computed at submission against the user's current position. For the exemption to be safe, the protocol needs that decomposition to still hold at fill time — otherwise the user could pass the submission check as a "pure reduction" and later fill against a different position state, ending up with a sub-min open position the value floor was supposed to prevent.

Two cases:

- **Market and IOC orders** fill in the same transaction as submission, so the decomposition is exact. The pure-reduction exemption applies whenever the decomposition is purely closing.
- **Resting limits (GTC, PostOnly)** can fill against any future position state — the user might open or close other positions between submission and fill. The submission-time decomposition is just a prediction. For these orders the exemption applies *only* when the `reduce_only` flag is set, because that flag is enforced at fill time by truncating the opening portion to zero. It guarantees the order can never grow exposure regardless of how the user's position changes between submission and fill.

Coverage table for **market and IOC** orders (decomposition known at fill):

| Scenario                                              | `closing` | `opening` (post-truncation) | `min_order_value` check |
| ----------------------------------------------------- | --------- | --------------------------- | ----------------------- |
| Long 100, sell 30 (non-reduce-only)                   | $-30$     | 0                           | exempt — pure reduction |
| Long 100, sell 100 (full close)                       | $-100$    | 0                           | exempt — pure reduction |
| Long 100, sell 150 (non-reduce-only, flip to short)   | $-100$    | $-50$                       | enforced                |
| Long 100, sell 150 (reduce-only, truncates to close)  | $-100$    | 0                           | exempt — pure reduction |
| Long 0, buy 50 (open from flat)                       | 0         | 50                          | enforced                |
| Long 100, buy 20 (increase long)                      | 0         | 20                          | enforced                |

For **GTC / PostOnly** limits, the rule collapses to "exempt iff `reduce_only` is set":

| Scenario                                              | `reduce_only` | `min_order_value` check                     |
| ----------------------------------------------------- | ------------- | ------------------------------------------- |
| Long 100, limit sell 30 (any decomposition)           | true          | exempt — fill-time truncation guarantees it |
| Long 100, limit sell 30                               | false         | enforced — submission decomposition unsafe  |
| Long 100, limit sell 150                              | true          | exempt — overshoot truncates at fill        |
| Long 100, limit sell 150                              | false         | enforced                                    |
| Long 0, limit buy 50                                  | either        | enforced                                    |

#### Implications of the exemption

- **Liquidations and ADL.** Every liquidation close is a pure reduction on the liquidated user's side ($\mathtt{openingSize} = 0$ by construction). The exemption applies automatically; no special-case plumbing is needed in the liquidation path.
- **Triggered conditional (TP/SL) orders.** The conditional-order cron submits market orders with `reduce_only = true`, which forces $\mathtt{openingSize} = 0$. A TP/SL trigger on a sub-min position therefore closes cleanly — no silent evaporation, no need for a snap-to-nearest rule.
- **Mixed orders that flip position direction.** Still subject to the check, on the *full* order notional. This matches Hyperliquid's behavior, where a flip order must clear `MinTradeNtl` against the full size.
- **Vault quoting.** The vault's `compute_bid` / `compute_ask` already check that each proposed quote's notional clears `min_order_value` before placing it. That logic is independent of the close-exemption rule, since vault quotes are always opening orders against the vault's own state.

#### Industry alignment

Hyperliquid's `MinTradeNtl` (\$10) applies to opens, but their platform documents — and [user reports](https://github.com/hummingbot/hummingbot/issues/6757) confirm — that *closing* a sub-\$10 position is permitted. Dango's pure-reduction rule generalizes that to any reduction, full or partial.

#### Choosing `min_order_value`

Two reasonable strategies:

- **Match one lot at a typical oracle price.** If `lot_size` is calibrated so one lot is worth ~\$5 at typical prices, set `min_order_value = $5`. New users can submit single-lot orders without hitting the value floor at normal prices; the floor only fires after a meaningful oracle drop.
- **Require multiple lots.** Set `min_order_value` to several lots' worth of notional (e.g., \$10–\$50). Stricter — discourages tiny gas-spam orders even at typical prices, at the cost of excluding small-account traders.

Set `min_order_value = 0` to disable the value floor entirely (lot size remains active). This is appropriate during initial chain bring-up but should be tightened before mainnet.

### 4.4 Tick size

The minimum price increment. Too small increases book fragmentation; too large creates implicit spread. Rule of thumb:

$$
\mathtt{tickSize} \approx \mathtt{oraclePrice} \times 10^{-4} \text{ to } 10^{-3}
$$

For BTC at \$60,000: tick sizes of \$1–\$10 are reasonable.

## 5. Vault parameters

The vault's market-making policy is controlled by three per-pair parameters and two global parameters (see [Vault](5-vault.md) for mechanics):

### 5.1 Half-spread

The half-spread should be calibrated to short-term intraday volatility so the vault earns a positive edge:

$$
\mathtt{halfSpread} \geq \sigma_{\mathtt{intraday}} + \mathtt{makerFeeRate}
$$

where $\sigma_{intraday}$ is the standard deviation of intra-block price changes. A larger spread protects against adverse selection but reduces fill probability.

### 5.2 Max quote size

Caps the vault's resting order size per side per pair. Should be consistent with `max_abs_oi` — the vault should not be able to accumulate more exposure than the system can handle:

$$
\mathtt{maxQuoteSize} \leq \mathtt{maxAbsOI} \times f, \quad f \in [0.3,\; 0.8]
$$

### 5.3 Liquidity weight

Determines what fraction of total vault margin is allocated to each pair. Higher-volume, lower-risk pairs should receive higher weights. The sum of all weights equals `vault_total_weight`.

### 5.4 Cooldown period

Prevents LPs from front-running known losses. Should exceed the funding period and be long enough that vault positions cannot be manipulated by short-term deposit/withdraw cycles. Typical values: 7–14 days.

## 6. Operational limits

| Parameter         | Calibration guidance                                                                                           |
| ----------------- | -------------------------------------------------------------------------------------------------------------- |
| `max_unlocks`     | Number of concurrent withdrawal requests per user. 5–10 is typical; prevents griefing with many small unlocks. |
| `max_open_orders` | Maximum resting limit orders per user across all pairs. 50–200; prevents order-book spam.                      |

## 7. Calibration workflow

The following checklist produces a complete parameter set from scratch:

1. **Collect data** — Gather ≥ 1 year of daily and hourly OHLCV data for each asset.

2. **Compute volatility** — For each asset, compute $r_{99.5}$ (daily 99.5th percentile absolute return) and $\sigma_{intraday}$ (hourly return standard deviation).

3. **Set margin ratios** — Derive MMR from $r_{99.5}$ ([§1.1](#11-volatility-based-derivation)), then IMR as a multiple of MMR. Cross-check against peer benchmarks ([§1.2](#12-peer-benchmarks)).

4. **Set fees** — Choose maker/taker/liquidation fee rates satisfying [§2.1](#21-sizing-principles) and the invariant in [§1.3](#13-invariants).

5. **Set funding** — Pick `funding_period`, derive `max_abs_funding_rate` ([§3.1](#31-max-funding-rate)), and calibrate `impact_size` ([§3.2](#32-impact-size)).

6. **Size exposure** — Set `max_abs_oi` from vault equity and tail-risk tolerance ([§4.1](#41-max-open-interest)).

7. **Set order constraints** — Choose `lot_size` ([§4.2](#42-lot-size)), `min_order_value` ([§4.3](#43-min-order-value)), and `tick_size` ([§4.4](#44-tick-size)). Calibrate `lot_size` and `min_order_value` together so that one lot at a typical oracle price clears `min_order_value` comfortably.

8. **Configure vault** — Set `vault_half_spread`, `vault_max_quote_size`, and `vault_liquidity_weight` per pair ([§5](#5-vault-parameters)), and `vault_cooldown_period` globally.

9. **Backtest** — Replay historical price data through the parameter set. Verify:
   - Liquidations occur before bad debt in > 99% of cases.
   - Vault PnL is positive over the test period.
   - Funding rates do not hit the clamp for more than 5% of periods.

10. **Deploy conservatively** — Launch with the conservative profile (lower leverage, higher fees, lower OI caps). Tighten parameters toward the aggressive profile as the system proves stable and liquidity deepens.
