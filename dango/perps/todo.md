# Perps Spec: Outstanding Issues

Issues identified during spec review that are not yet addressed. Ordered roughly by severity.

## Critical

- ~~**No trading fees collected.** The intro states the pool profits from "taking a cut from trading fees," but no fee is ever collected in `execute_fill` or anywhere else. Industry standard: 0.01%-0.1% maker/taker or open/close fee on notional value. This is a critical revenue source for the vault.~~
  - Fixed. Added `trading_fee_rate` to `Params` and `collect_trading_fee` helper. Fee collected on every voluntary fill (market + limit), exempt during liquidation.

- ~~**No maintenance margin ratio.** Only `initial_margin_ratio` exists. Every major exchange defines a separate, lower `maintenance_margin_ratio` (e.g. 2.5% vs 5% initial). Without it, there's no well-defined liquidation trigger. The Liquidation section cannot be properly specified without this parameter.~~
  - Fixed. Added `maintenance_margin_ratio` to `PairParams` and `liquidation_fee_rate` to `Params`. Liquidation section fully specified with `handle_force_close`.

- ~~**Unrealized PnL not factored into equity for margin checks.** `compute_available_margin` uses raw `user_state.margin` (deposit balance) without adding unrealized PnL. In cross-margin mode, the standard is: `equity = balance + unrealized_pnl`, and `available = equity - used - reserved`. A user with 1000 USDT balance and -800 USDT unrealized loss currently shows ~500 available margin (1000 - 500 used), when it should be ~200.~~
  - Fixed. `compute_available_margin` now uses `compute_user_equity` (margin + unrealized PnL - accrued funding) instead of raw margin.

## High

- ~~**No oracle staleness check.** The DEX module uses `MAX_ORACLE_STALENESS = 500ms`. The perps spec should reference a similar safeguard -- stale oracle prices in a leveraged perps exchange are far more dangerous than in spot DEX.~~
  - Oracle staleness check is encapsulated in the `OracleQuerier` helper struct.

- ~~**Vault withdrawal doesn't guard against negative equity.** `handle_unlock_liquidity` computes `amount_to_release = floor(vault_equity * shares / supply)`. If vault_equity is negative (insolvency), this underflows or returns 0, but the function doesn't explicitly check. The deposit side checks for this (`vault_equity > 0`), but withdrawal should too.~~
  - Fixed.

## Medium

- **`cost_basis` rounding drift on repeated partial closes.** Each partial close does `floor(cost_basis * (1 - close_ratio))`, accumulating rounding errors. After many small closes, the remaining cost_basis can diverge meaningfully from the true proportional entry cost. Consider tracking `entry_price` alongside cost_basis, or computing cost_basis from entry_price on demand.

- ~~**No minimum position/order size.** _(Already noted as TODO in spec.)_ Without this, dust positions can grief the system -- tiny positions that cost more gas to liquidate than they're worth.~~
  - Fixed. Added `min_order_notional` and `min_position_notional` to `PairParams`.

- ~~**No maximum open orders per user.** _(Already noted as TODO in spec.)_ Without this, a user can create an unbounded number of limit orders, bloating storage.~~
  - Fixed. Added `max_open_orders` to `Params` and `open_order_count` to `UserState`.
