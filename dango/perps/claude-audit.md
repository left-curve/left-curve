# Perps Specification Review

## Category 1: Vulnerabilities to Hacks or Economic Exploits

### 1.1 [CRITICAL] Vault Share Inflation / First Depositor Attack

**Fixed.** Replaced `DEFAULT_SHARES_PER_AMOUNT` with `VIRTUAL_SHARES` / `VIRTUAL_ASSETS` virtual offset pattern in both `handle_deposit_liquidity` and `handle_unlock_liquidity`.

~~When `vault_share_supply == 0`, the first depositor receives `floor(amount * DEFAULT_SHARES_PER_AMOUNT)` shares. An attacker can:~~

1. ~~Deposit 1 wei of USDT, receiving `DEFAULT_SHARES_PER_AMOUNT` shares~~
2. ~~Open a position and intentionally lose (their loss goes to `vault_margin` via `settle_pnl`), inflating vault equity while share supply is tiny~~
3. ~~Next depositor who sends, say, 999 USDT gets `floor(999 * 1_000_000 / 1_000_001) = 0` shares (floor rounding), and the attacker redeems all shares for the entire vault~~

~~This is the well-known ERC-4626 vault attack. **Fix:** Use a "virtual offset" or "dead shares" pattern. On first deposit, mint dead shares to the zero address, or use `shares = (amount * (totalSupply + OFFSET)) / (equity + OFFSET)`.~~

### 1.2 [HIGH] Unbounded Funding Rate Can Create Bad Debt Spirals

**Fixed.** Added `max_abs_funding_rate` to `PairParams` and clamped the computed rate in `compute_current_funding_rate`.

~~The spec explicitly does not clamp the funding rate: "the rate is NOT clamped. It can grow unboundedly." This creates severe risks:~~

~~- The funding rate accelerates quadratically over time (rate = integral of velocity). A persistent skew can make the rate so extreme that the majority side's equity is wiped out in hours, causing cascading liquidations.~~
~~- An attacker can build up skew, let the rate grow, then open a minority-side position to collect enormous funding from the vault.~~
~~- Most production systems (Synthetix V3, dYdX, Hyperliquid) clamp the rate. Synthetix V2's unclamped rate was a source of issues.~~

~~**Fix:** Add `max_abs_funding_rate` to `PairParams` and clamp the rate after the velocity update.~~

### 1.3 [HIGH] No Margin Revalidation on Limit Order Fill

**Fixed.** Added a margin check (step 5) in `try_fill_limit_order` between the OI check and price check. If the user's equity can no longer support the order after accounting for projected used margin and trading fees, the order is cancelled and its reserved margin is released.

~~`try_fill_limit_order` explicitly skips margin checks: "No margin check is performed here." But between placement and fill:~~

~~- The user's equity may have collapsed (losses on other positions, funding accrual)~~
~~- Oracle price may have diverged far from limit_price, making `used_margin` (oracle-based) much larger than the margin reserved at placement (limit_price-based)~~

~~Example: User places buy limit at 100, reserving 5 USDT margin (5% of 100). By fill time, oracle is at 200. Used margin becomes 10 USDT. Position is immediately under-margined. **Fix:** Validate margin at fill time; cancel the order if the user can no longer afford it.~~

### 1.4 [HIGH] Skew Manipulation for Liquidation MEV

**Won't fix.** `max_abs_premium` already hard-caps how much skew can affect execution price — the audit's "extreme skew" scenario is bounded by this parameter (the same parameter motivated by the Mars Protocol hack). The audit's claim that "Binance/OKX/dYdX use oracle price for liquidation execution" is factually wrong: all major exchanges use oracle/mark price only for _triggering_ liquidation, then execute at order book fill prices (Binance uses IOC orders against the book, dYdX v4 matches protocol orders against the book, Hyperliquid sends market orders to the book). The spec's skew-adjusted pricing is the AMM equivalent of order book execution, and liquidation triggers already use oracle-based equity (`is_liquidatable`). The spec comments at lines 1789-1799 explicitly justify this design: skew-adjusted liquidation pricing prevents moral hazard where users would prefer being liquidated over voluntarily closing when the skew premium makes voluntary closure more expensive.

~~Liquidation trigger uses oracle price (correct), but execution uses skew-adjusted price. An attacker can:~~

1. ~~Build a large position to create extreme skew~~
2. ~~Liquidate a user whose positions close unfavorably due to the skew premium~~
3. ~~Unwind their position~~

~~This extracts extra value from the liquidated user beyond what oracle price justifies. **Fix:** Use oracle price for liquidation execution (as Binance/OKX/dYdX do), or use a tighter premium cap for liquidation fills.~~

### 1.5 [HIGH] Reserved Margin Drift When Positions Change

**Fixed.** Stored exact `reserved_margin` per `Order` so fill and cancel release exactly what was reserved, eliminating accounting drift.

~~When a limit order is placed, `decompose_fill` determines closing vs. opening based on the user's current position, and margin is reserved for the opening portion. But if other trades change the position before the limit order fills, the decomposition is stale:~~

~~1. User has +100. Places sell limit -150 (closing=-100, opening=-50). Reserves margin for opening=-50.~~
~~2. A market order reduces position to +20.~~
~~3. Now the limit's actual decomposition would be closing=-20, opening=-130. Reserved margin only covers -50.~~

~~**Fix:** Store the reserved amount directly in each `Order` struct, and revalidate margin at fill time. Alternatively, cancel limit orders when the user's position in that pair changes.~~

### 1.6 [MEDIUM] Vault Share Sandwich on Deposits

**Won't fix.** The audit note does not specify how the attack can "extract value".

~~An LP can observe a pending `DepositLiquidity` tx and manipulate vault equity (via trading, oracle timing) to extract value. The `min_shares_to_mint` parameter helps but requires the user to compute the correct value off-chain.~~

~~**Fix:** Add a deposit cooldown, time-weighted share pricing, or a deposit cap per block.~~

### 1.7 [MEDIUM] Bad Debt with No Auto-Deleveraging

**Fixed.** Added `adl_trigger_ratio` parameter and dedicated `handle_adl` function that forcibly closes a single user position when vault equity falls below the trigger ratio. Admin-only (offchain bot) with per-position granularity and profitability-based priority ranking.

~~Bad debt is "absorbed by the vault" via `saturating_sub` in `settle_pnl`. Without ADL (acknowledged as future work), if vault equity goes to zero, deposits and withdrawals both freeze (`ensure!(vault_equity > 0)`), with no recovery path. Every major perps exchange has ADL.~~

~~**Fix:** Implement ADL before launch. The existing `handle_force_close` with admin-only bypass is a start but needs priority ordering (highest-profit positions first).~~

### 1.8 [MEDIUM] Oracle Price Manipulation Affects Vault Equity

**Won't fix.** Out of scope for this spec.

~~`compute_vault_equity` depends on oracle prices for all pairs. Manipulating the oracle for a single thin-market pair changes vault equity, affecting deposits/withdrawals across the entire vault.~~

~~**Fix:** TWAP oracles, multi-source aggregation, per-pair unrealized PnL caps for vault equity calculations, and circuit breakers on vault operations during sharp price moves.~~

### 1.9 [CRITICAL] `unfilled_size` Miscalculated When Price Constraint Fails

**Fixed.** Restructured `handle_submit_order` so that when a limit order's price constraint fails, the full order is stored in the book via early return (step 6). `unfilled_size` is only computed in the success path, eliminating the silent order loss.

~~In `handle_submit_order` (lines 1008-1018):~~

~~```rust~~
~~if fill_size != Dec::ZERO {~~
~~    let exec_price = compute_exec_price(...);~~
~~    if check_price_constraint(exec_price, target_price, is_buy) {~~
~~        execute_fill(...);  // only runs if price check passes~~
~~    }~~
~~}~~
~~let unfilled_size = size - fill_size;  // always subtracts fill_size~~
~~```~~

~~If `fill_size != 0` but the price constraint fails, `execute_fill` is _not_ called, yet `unfilled_size = size - fill_size` still deducts the unattempted fill. For **limit orders**, this means part of the order is silently lost: neither filled nor stored in the book.~~

~~Example: User submits a limit sell for -150, with existing position +100. `closing_size=-100`, `opening_size=-50`. OI is fine, so `fill_size=-150`. Exec price fails the constraint. Fill doesn't execute. `unfilled_size = -150 - (-150) = 0`. No limit order is stored. The entire order vanishes silently, with no error.~~

~~**Fix:** Track whether the fill actually executed:~~

~~```rust~~
~~let actually_filled = if fill_size != Dec::ZERO {~~
~~    let exec_price = compute_exec_price(...);~~
~~    if check_price_constraint(exec_price, target_price, is_buy) {~~
~~        execute_fill(...);~~
~~        fill_size~~
~~    } else { Dec::ZERO }~~
~~} else { Dec::ZERO };~~
~~let unfilled_size = size - actually_filled;~~
~~```~~

### 1.10 [HIGH] `reduce_only` Does Not Restrict to Closing-Only

**Fixed.** `handle_submit_order` now zeros out `opening_size` at step 2 when `reduce_only=true` (spec line 1012), before the OI check. This ensures reduce-only orders can only fill the closing portion regardless of OI state.

~~The API documentation (line 99) says: "If true, only the closing portion of the order is executed; any opening portion is discarded." But `compute_fill_size_from_oi` (line 898) only applies this when OI is violated:~~

~~```rust~~
~~if oi_violated {~~
~~    if reduce_only { closing_size } else { Dec::ZERO }~~
~~} else {~~
~~    size  // full size regardless of reduce_only!~~
~~}~~
~~```~~

~~When OI is _not_ violated, a `reduce_only=true` order can open new exposure. This contradicts the spec's own documentation and every major exchange's definition of reduce-only (Binance, Bybit, dYdX, Drift all restrict reduce-only to closing regardless of OI state).~~

~~**Fix:** When `reduce_only=true`, always return only the closing portion:~~

~~```rust~~
~~if reduce_only {~~
~~    closing_size~~
~~} else if oi_violated {~~
~~    Dec::ZERO~~
~~} else {~~
~~    size~~
~~}~~
~~```~~

### 1.11 [HIGH] Self-Liquidation to Avoid Trading Fees

**Fixed.** Liquidation fee is now paid to the vault instead of the liquidator, eliminating the self-liquidation exploit.

~~Liquidation fills are exempt from trading fees (line 1264: "the existing `liquidation_fee_rate` is the only fee on liquidation"). A user can exploit this:~~

~~1. Withdraw margin to just below maintenance margin.~~
~~2. Call `force_close` on themselves from a second account (the "liquidator").~~
~~3. All positions are closed with no trading fee. The liquidation fee goes to their own second account.~~

~~Net cost: zero (liquidation fee is self-paid). Savings: `trading_fee_rate * total_notional`.~~

~~**Fix options:**~~

~~- Charge trading fees on liquidation fills as well (in addition to the liquidation fee).~~
~~- Add a minimum time between margin withdrawal and liquidation eligibility.~~
~~- Pay the liquidation fee to the vault (not the liquidator) with a separate, smaller keeper incentive.~~

### 1.12 [MEDIUM] Withdrawal Amount Locked at Unlock Time, Not Claim Time

**Won't fix.** Intended design.

~~In `handle_unlock_liquidity` (line 2037), `amount_to_release` is computed immediately and stored in the `Unlock` struct. During the cooldown period, the vault may suffer losses. The withdrawing LP still receives their pre-loss amount, socializing the loss to remaining LPs.~~

~~Scenario: LP observes a large position approaching liquidation. LP unlocks at current (healthy) equity. During cooldown, the position is liquidated and the vault absorbs bad debt. LP claims the pre-loss amount, effectively externalizing the loss to remaining LPs.~~

~~**Fix:** Recompute `amount_to_release` at claim time, or use the minimum of unlock-time and claim-time valuations. The shares should be the unit stored in `Unlock`, not a pre-computed USDT amount.~~

---

## Category 2: Deviations from Industry Best Practices

### 2.1 [HIGH] Full Liquidation Only

**Won't fix.** Planned for V2.

~~The spec liquidates ALL positions across ALL pairs. Every major exchange (Binance, Bybit, dYdX, Hyperliquid, Drift) uses partial liquidation: close positions incrementally until solvency is restored. Full liquidation is unnecessarily destructive -- it closes winning positions alongside losing ones, creates larger market impact, and triggers more cascading liquidations.~~

~~Acknowledged as v2, but launching without it is a significant deviation from industry norms.~~

### 2.2 [MEDIUM] No Maker/Taker Fee Split

**Won't fix.** Not applicable for peer-to-pool protocols, where the user is always the taker and the pool is always the maker, so one fee rate suffices.

~~Flat `trading_fee_rate` for all fills. Industry standard differentiates maker (resting limit orders) from taker (market/immediately-filled) fees. Binance, dYdX, Hyperliquid all differentiate. Maker rebates incentivize limit order placement and improve liquidity.~~

### 2.3 [MEDIUM] No Per-User Position Limits

**Won't fix.** Attackers can easily circumvent this by creating multiple accounts.

~~The only constraint is the global `max_abs_oi` per pair. A single whale can consume the entire OI capacity, concentrating vault risk on one counterparty. Binance, dYdX, and Hyperliquid all have per-user position limits.~~

### 2.4 [MEDIUM] No Price Bands or Circuit Breakers

**Won't fix.** Oracle is a separate smart contract, out of the scope of this spec.

~~No oracle staleness check, no max price change per update, no trading halt mechanism. An oracle malfunction could trigger mass liquidations at incorrect prices. Binance has price bands; CME has limit-up/limit-down halts; dYdX has oracle staleness checks; Synthetix V3 has dynamic volatility fees.~~

### 2.5 [MEDIUM] No Keeper Incentive for Limit Order Processing

**Won't fix.** Limit order processing is automatically triggered. No offchain keeper involved.

~~Limit orders are fulfilled "at the beginning of each block" but there's no explicit incentive for whoever pays the gas to trigger this processing. Synthetix, dYdX, and Drift all have keeper networks with gas reimbursement and incentive fees.~~

### 2.6 [LOW] No GTT (Good-Til-Time) or Post-Only Orders

**Won't fix.** We don't see the demand for this for now.

~~Only IOC (market) and GTC (limit). GTC orders with no expiry linger in storage indefinitely. Most exchanges support time-limited orders and post-only mode.~~

### 2.7 [MEDIUM] `used_margin` for Existing Positions Uses `initial_margin_ratio`

**Dismissed.** The current design matches industry standard (dYdX, Binance): `initial_margin_ratio` is used for available-balance and withdrawal checks; `maintenance_margin_ratio` is used only for the liquidation trigger. Using initial margin for existing positions is correct.

~~`compute_used_margin` (line 574) uses `initial_margin_ratio` for existing open positions. Most exchanges distinguish between the margin required to _open_ a position (initial margin) and the margin _held_ against an existing position (often a lower rate, closer to maintenance). Using the higher `initial_margin_ratio` for existing positions means available margin is more restricted than necessary, potentially preventing users from placing closing orders or withdrawing funds when they're in drawdown but still well above maintenance.~~

### 2.8 [LOW] Limit Order Fulfillment Uses Marginal Price Cutoff

**Dismissed.** We want to find all fillable orders, even if we visit from unfillable ones.

~~In `fulfill_limit_orders_for_pair` (lines 1323-1329), the fillability check compares `limit_price` against `marginal_price` (the execution price for an infinitesimal order). But the actual execution uses `compute_exec_price` which includes the order's own size impact. An order that passes the marginal price check may still fail the exec price check (handled in `try_fill_limit_order`). This means the loop may evaluate many orders that ultimately can't fill, wasting gas. Not incorrect, but suboptimal.~~

---

## Category 3: Inefficiencies

### 3.1 [MEDIUM] `compute_vault_equity` Iterates All Pairs Twice

**Dismissed.** In this spec, we write too loops for better readability. We've added a comment suggesting they can be combined into a single loop in the actual implementation.

~~`compute_vault_unrealized_pnl` and `compute_vault_unrealized_funding` each iterate all `pair_states` separately. These could be a single pass:~~

~~```rust~~
~~fn compute_vault_unrealized_totals(...) -> (Dec, Dec) {~~
~~    let mut total_pnl = Dec::ZERO;~~
~~    let mut total_funding = Dec::ZERO;~~
~~    for (pair_id, pair_state) in pair_states {~~
~~        // compute both in one iteration~~
~~    }~~
~~    (total_pnl, total_funding)~~
~~}~~
~~```~~

### 3.2 [MEDIUM] Redundant `decompose_fill` Calls

**Fixed.** `execute_fill` now takes `closing_size` and `opening_size` as parameters; callers pass the already-computed decomposition.

~~`handle_submit_order` calls `decompose_fill` at step 1 (line 984), then `execute_fill` calls it again internally (line 1111). Same redundancy in `try_fill_limit_order`. Pass the decomposition as parameters instead.~~

### 3.3 [LOW] Liquidation Iterates Positions Twice

**Fixed.** Merged the two position loops in `handle_force_close` into a single loop that computes `total_notional` while closing positions.

~~`handle_force_close` iterates positions once for `total_notional` (lines 1774-1777) and once to close them (lines 1797-1808). Combine into a single loop.~~

### 3.4 [LOW] Lazy Funding Accrual Could Be Eagerly Batched

**Fixed.** We added an early-return condition in `accrual_funding` such that if no time has passed since the last funding accrual, then the operation is skipped. This ensures for each pair, funding is only accrued once per block. This achieves the same optimization as the active accrual suggested.

~~Funding is accrued lazily per-pair when user operations touch that pair. Accruing all active pairs once at block start (when oracle prices arrive) would eliminate redundant accruals when multiple users interact with the same pair in one block.~~

### 3.5 [LOW] Margin Estimated at `target_price`, Charged at `exec_price`

**Dismissed.** No longer applicable; the current spec uses `exec_price` for both the margin check and the actual fee.

~~In `handle_submit_order` step 5, the margin check uses `target_price` (worst-case) for the estimated fee, but the actual fee uses the (typically better) `exec_price`. The overestimate is conservative and safe, but may reject valid orders that would have had sufficient margin at the actual execution price.~~

### 3.6 [LOW] Limit Order Book Scanned Every Block

**Dismissed.** We only scan orders that can be potentially filled. If an order with an unfavorable price is reached, the scanning is terminated. If the oracle price doesn't meaningfully change, then the number of eligible orders should also be small.

~~`fulfill_limit_orders_for_pair` scans both order books every block for every pair. If most blocks don't have meaningful oracle price changes, this is wasted computation. Consider triggering scans only when oracle prices change beyond a threshold, or maintaining a price-indexed trigger per pair.~~

---

## Category 4: Code Quality Issues

### 4.1 [HIGH] `execute_fill` Is Too Complex

`execute_fill` (lines 1099-1195) handles 9 distinct responsibilities: decomposition, two accumulator updates, funding settlement, PnL settlement, position size update, entry price blending, position creation/deletion, and OI bookkeeping. The accumulator update ordering is critical and fragile (acknowledged in the test case notes).

**Fix:** Break into smaller functions: `settle_existing_position(...)`, `apply_fill_to_position(...)`, `update_oi(...)`, each with clear pre/postconditions.

### 4.2 [MEDIUM] `oi_weighted_entry_funding` Is Modified 4 Times Per Fill

`settle_funding` removes old and adds new contribution (2 ops), then `execute_fill` removes post-settlement contribution and adds final contribution (2 more ops). This is correct but extremely confusing. `oi_weighted_entry_price` is handled more cleanly (1 remove + 1 add). **Fix:** Have `settle_funding` only handle the fund transfer and entry point update; let `execute_fill` handle all accumulator updates consistently.

### 4.3 [MEDIUM] `settle_pnl` Rounding Inconsistency for Negative PnL

The stated principle is "floor when advantaging user, ceil when advantaging protocol." But for negative PnL:

```rust
let loss = floor(-pnl);  // pnl < 0, so -pnl > 0
```

`floor(-pnl)` = `floor(|pnl|)` means the user pays _less_ than the exact loss. This advantages the **user**, contradicting the principle. Should be `ceil(-pnl)` so the protocol collects at least the full amount.

### 4.4 [MEDIUM] Margin Check Blocks reduce_only Close Orders

In `handle_submit_order`, the margin check (step 5) uses the original `opening_size` from step 1. But if `reduce_only=true` and OI is violated, the opening portion is discarded (step 3). A user trying to close via reduce_only may be rejected because the margin check runs on the full opening portion that will never execute.

Example: User has +100, submits sell -150 with reduce_only=true. Opening=-50 requires margin. If available margin is insufficient, the order is rejected -- even though only the closing=-100 portion would actually execute. **Fix:** Either move the margin check after fill size determination, or skip the margin check when the effective fill is close-only.

### 4.5 [MEDIUM] `short_oi` as Non-Positive Is Confusing

Storing `short_oi` as a negative number (e.g., -100) requires `.abs()` calls, sign-aware comments, and non-obvious formulas like `pair_state.short_oi += closing_size` (adding positive to negative to make it "less negative"). Storing it as positive `Udec` with `skew = long_oi - short_oi` is more standard and eliminates a class of sign errors.

### 4.6 [MEDIUM] Missing Parameter Validation

**Won't fix.** We assume the admin won't make this problem.

~~`maintenance_margin_ratio < initial_margin_ratio` is stated in comments but never enforced. Other important invariants also lack programmatic enforcement: `skew_scale > 0`, `max_abs_premium < 1`, `initial_margin_ratio > 0`, `trading_fee_rate < 1`. A misconfiguration could make trading impossible or cause immediate liquidation.~~

### 4.7 [LOW] `cancel_order` Recomputes Reserved Margin Instead of Storing It

**Fixed (as part of 1.5).** `Order` now stores `reserved_margin`; cancel releases the stored amount exactly.

~~Cancellation recomputes reserved margin using the current position (which may have changed), then uses `saturating_sub`. This means `reserved_margin` can drift from the true sum of outstanding reservations. **Fix:** Store the reserved amount in each `Order` struct.~~

### 4.8 [LOW] Test Cases Use `cost_basis` but Spec Uses `entry_price`

**Fixed.** Simply removed test cases.

~~The vault unrealized PnL test cases reference `cost_basis` (from the old spec), while the current `Position` struct uses `entry_price` (per commit `b62691cb0`). Update test cases for consistency.~~

### 4.9 [LOW] No Events/Logging Specified

**Dismissed.** Out of scope for this spec.

No event structures are defined for fills, liquidations, deposits, withdrawals, or funding accruals. These are essential for off-chain indexing, frontends, and monitoring.

### 4.10 [MEDIUM] Type Error in `PAIR_PARAMS` Storage Layout

**Fixed.**

~~Line 464:~~

```rust
const PAIR_PARAMS: Map<PairId, Params> = Map::new("pair_params");
```

~~Should be `Map<PairId, PairParams>`. `Params` is the global parameters struct; `PairParams` is the per-pair parameters struct.~~

### 4.11 [MEDIUM] Function Call/Signature Mismatches

`handle_deposit_liquidity` (line 1975) calls `compute_vault_equity(state, usdt_price)` with 2 arguments, but the function signature (line 1855) requires 6 parameters (`state`, `pair_states`, `pair_params_map`, `oracle_prices`, `usdt_price`, `current_time`). Same issue in `handle_unlock_liquidity` (line 2025).

`compute_vault_unrealized_funding` (line 1894) has `usdt_price` in its signature but doesn't use it, and the call in `compute_vault_equity` (line 1865) doesn't pass it.

### 4.12 [MEDIUM] `usdt_price` Inconsistency Between User and Vault Equity

`compute_vault_equity` (line 1872) divides unrealized PnL/funding by `usdt_price` to convert from USD to USDT terms. But `compute_user_equity` (line 620) does _not_ apply any `usdt_price` conversion. If oracle prices are denominated in USD and the settlement currency is USDT, both should apply the same conversion. A USDT depeg would create an accounting mismatch between user equity and vault equity.

### 4.13 [MEDIUM] Rounding Direction Error in `compute_used_margin`

Line 588: `total += floor(margin);`

Used margin is floored, making it _smaller_, giving the user _more_ available margin (`available = equity - used - reserved`). This is _advantageous to the user_, contradicting the spec's stated rounding principle (line 1986): "Round the number down, to the advantage of the protocol."

Should be `ceil(margin)` to be conservative (less available margin = harder to over-leverage). Note: `compute_maintenance_margin` (line 1670) correctly uses `ceil` for the same type of calculation.

### 4.14 [LOW] Inconsistent Naming: `BIDS`/`ASKS` vs `BUY_ORDERS`/`SELL_ORDERS`

**Fixed.**

~~Storage layout (lines 479/486) defines `BIDS` and `ASKS`, but `try_fill_limit_order` (line 1381) references `BUY_ORDERS`/`SELL_ORDERS`.~~

### 4.15 [LOW] Redundant `Direction` in Order Book Keys

**Fixed.**

~~Line 479: `BIDS: IndexedMap<(PairId, Direction, Udec, Timestamp, OrderId), Order>`~~

~~Since `BIDS` only contains buys and `ASKS` only contains sells, including `Direction` in the composite key is redundant and wastes storage/gas on every key operation.~~

### 4.16 [LOW] Undefined `MAX_PRICE` Constant

**Fixed.** Replaced `MAX_PRICE` with `Udec::MAX`.

~~`MAX_PRICE` is used (lines 1084, 1324, 1377) but never defined. Its value and the assumption that all valid prices are strictly less than it should be specified.~~

### ~~4.17 [LOW] Typo in Error Message~~

**Fixed.** ~~Line 1997: `"to few shares would be minted"` should be `"too few shares would be minted"`.~~
