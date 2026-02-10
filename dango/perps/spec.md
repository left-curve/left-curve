# Perpetual futures exchange: specifications

- This is a perpetual futures (perps) exchange that uses the **peer-to-pool model**, similar to e.g. Ostium, Synthetix V3, and Gains Network. A liquidity pool provides quotes (based on oracle price, open interest (OI), and the order's size). All orders are executed against the pool, with the pool taking the counterparty position (e.g. if a user opens a long position of 5 BTC, the pool takes the opposite: a short position of 5 BTC). We call the pool the **counterparty vault**. This is in contrary to the peer-to-peer model, where users place orders in an order book; a user's order is executed against other users' orders. The pool makes profit in two way: from users in aggregate losing (which is the case over the long run, empirically), and taking a cut from trading fees.
- The exchange operates in **one-way mode**. Meaning, e.g., a user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, we modify the existing position, insteading of creating a new one. This is in contrary to **two-way mode**, where a user can have multiple positions in a single asset; when placing an order, the user can choose whether the order will create a new position or modify an existing one.
- To ensure the protocol's solvency and profitability, it's important the market is close to _neutral_, meaning there is roughly the same amount of long and short OI. We incentivize this through two mechanisms, **skew pricing** and **funding fee** (described in respective sections).
- For this v1 release:
  - Only **cross margin** is supported. Support for isolated margin may be added in a future update.
  - Order fulfillment is **all-or-nothing**: while the closing portion of an order can always be fully filled (bypassing max OI constraint), the opening portion is either filled fully (if it satisfies the max OI constraint), or not at all (if it doesn't satisfy). Partial fill of orders may be added in a future release.

This spec is divided into three sections:

- _API_ defines the methods available for users to interact with the smart contract.
- _Storage_ defines what data are to be saved in the smart contract's storage.
- _Business logic_ defines how the smart contract should handle user requests through the API.

Regarding code snippets:

- Written in Rust-like pseudocode.
- `Dec` is a signed fixed-point decimal number type. The size of perpetual futures contracts, the size of orders, etc. are represented by `Dec`.
- `Udec` is an unsigned fixed-point decimal number type. The oracle price is represented by `Udec`.
- `Uint` is an unsigned integer number type. The amount of the settlement currency and vault shares are represented by `Uint`.
- In the pseudocode we ignore the conversion between these number types, except for when converting from decimal to integer, where it's necessary to specify whether it should be floored or ceiled.
- We also assume all the math traits are implemented. E.g. we may directly multiply a `Dec` with a `Uint`.

## API

User may interact with the smart contract by dispatching the following execution message:

```rust
enum ExecuteMsg {
    // -------------------------------------------------------------------------
    // Counterparty Vault Methods
    // -------------------------------------------------------------------------

    /// Add liquidity to the counterparty vault.
    ///
    /// User must send a non-zero amount of the settlement currency (defined in
    /// `Params` below) as attachment to this function call.
    DepositLiquidity {
        /// Revert if less than this number of share is minted.
        min_shares_to_mint: Option<Uint>,
    },

    /// Request to withdraw funds from the counterparty vault.
    ///
    /// The request will be fulfilled after the cooldown period, defined in the
    /// global parameters.
    UnlockLiquidity {
        /// The amount of vault shares the user wishes to burn.
        /// Must be no more than the amount of vault shares the user owns.
        shares_to_burn: Uint,
    },

    // -------------------------------------------------------------------------
    // Trading Methods
    // -------------------------------------------------------------------------

    /// Deposit funds into the user's trading balance.
    ///
    /// ## Note
    ///
    /// In the actual implementation, the user's margin is simply the entire token
    /// balance managed by the bank contract. It isn't necessary to deposit tokens
    /// into the perps contract. But in this spec we make the deposit action explicit
    /// for clarity.
    DepositMargin {},

    /// Withdraw funds from the user's trading balance.
    ///
    /// Can only withdraw up to the available margin (total balance minus used
    /// margin minus reserved margin).
    ///
    /// ## Note
    ///
    /// In the actual implementation, this is equivalent to sending tokens away
    /// from one's account. The margin check can happen in the bank contract.
    WithdrawMargin {
        /// The amount of settlement currency to withdraw.
        amount: Uint,
    },

    /// Submit an order.
    SubmitOrder {
        // The pair ID can either be numerical or string-like.
        pair_id: PairId,

        /// The amount of the futures contract to buy or sell.
        ///
        /// E.g. when trading in the BTCUSD-PERP pair, if 1 BTCUSD-PERP futures
        /// contract represents 1 BTC, and the user specifies a `size` of +1, it
        /// means the user wishes to increase his long exposure or decrease his
        /// short exposure by 1 BTC.
        ///
        /// Positive for buy (increase long exposure, decrease short exposure);
        /// negative for sell (decrease long exposure, increase short exposure).
        size: Dec,

        /// The order type: market, limit, etc.
        kind: OrderKind,

        /// If true, only the closing portion of the order is executed; any
        /// opening portion is discarded. If false, the full order is
        /// attempted; if the opening portion violates OI constraints, only
        /// the closing portion fills.
        reduce_only: bool,
    },

    /// Cancel a pending limit order.
    ///
    /// Releases the margin reserved for this order back to available margin.
    CancelOrder {
        /// The pair ID of the order to cancel.
        pair_id: PairId,

        /// The order ID to cancel.
        order_id: OrderId,
    },

    /// Forcibly close all of a user's positions.
    ///
    /// This can happen during a liquidation (callable by anyone), or during
    /// auto-deleveraging (callable only by the administrator).
    ForceClose {
        /// The user's identifier. In the smart contract implementation, this
        /// should be an account address.
        user: UserId,
    },
}

enum OrderKind {
    /// Trade at the current price quoted by the counterparty vault, optionally
    /// with a slippage tolerance.
    ///
    /// If it's not possible to fill the order in full, the unfilled portion is
    /// canceled.
    Market {
        /// The execution price must not be worse than the _marginal price_ plus
        /// this slippage.
        ///
        /// Marginal price is the price that the counterparty vault may quote for
        /// an order of infinitesimal size.
        ///
        /// For bids / buy orders, the execution price satisfy:
        ///
        /// ```plain
        /// exec_price <= marginal_price * (1 + max_slippage)
        /// ```
        ///
        /// For asks / sell orders:
        ///
        /// ```plain
        /// exec_price >= marginal_price * (1 - max_slippage)
        /// ```
        max_slippage: Udec,
    },

    /// Trade at the specified limit price.
    ///
    /// If it's not possible to fill the order in full, and the user hasn't
    /// reached the maximum open order count, the unfilled portion is persisted
    /// in the contract storage, either filled later when the necessary conditions
    /// are met or canceled by the user.
    Limit {
        /// The execution price must be equal to or better than this price.
        limit_price: Udec,
    },
}
```

## Storage

Storage is divided into _parameters_, which are set by the administrator and not changed by user operations (adding/removing liquidity, opening/closing orders, liquidation), and _state_, which are updated by user operations.

### Data structures

#### Parameters

The global parameters apply to all trading pairs:

```rust
struct Params {
    /// Denomination of the asset used for the settlement of perpetual futures
    /// contracts. Typically a USD stablecoin.
    pub settlement_currency: Denom,

    /// The waiting period between a withdrawal from the counterparty vault is
    /// requested and is fulfilled.
    pub vault_cooldown_period: Duration,

    /// Maximum number of resting limit orders a single user may have
    /// across all pairs. Prevents storage bloat from order spam.
    pub max_open_orders: u32,

    /// Trading fee as a fraction of notional value, charged on every fill.
    ///
    /// fee = |fill_size| * exec_price * trading_fee_rate
    ///
    /// Deducted from the user's margin and transferred to the vault.
    /// E.g., 0.0005 = 0.05%. For a $100,000 notional fill, fee = $50.
    pub trading_fee_rate: Udec,

    /// Fee paid to the vault as a fraction of the total notional value
    /// of positions being liquidated.
    ///
    /// E.g., 0.0005 = 0.05%. For $100,000 total notional, fee = $50.
    ///
    /// Capped at the user's remaining margin after position closure.
    /// Typical range: 0.0002 to 0.001 (0.02% to 0.1%).
    pub liquidation_fee_rate: Udec,
}
```

Each trading pair is also associated with a set of pair-specific parameters:

```rust
struct PairParams {
    /// A scaling factor that determines how greatly an imbalance in open
    /// interest ("skew") should affect an order's execution price.
    /// The greater the value of the scaling factor, the less the effect.
    pub skew_scale: Udec,

    /// The maximum extent to which skew can affect execution price.
    /// The execution price is capped in the range `[1 - max_abs_premium, 1 + max_abs_premium]`.
    /// This prevents an exploit where a trader fabricates a big skew to obtain
    /// an unusually favorable pricing.
    /// See the Mars Protocol hack: <https://x.com/neutron_org/status/2014048218598838459>.
    pub max_abs_premium: Udec,

    /// The maximum allowed open interest for both long and short.
    /// I.e. the following must be satisfied:
    ///
    /// - |pair_state.long_oi| <= pair_params.max_abs_oi
    /// - |pair_state.short_oi| <= pair_params.max_abs_oi
    ///
    /// This constraint does not apply to reducing positions.
    pub max_abs_oi: Udec,

    /// Maximum absolute funding rate, as a fraction per day.
    ///
    /// The funding rate is clamped to [-max_abs_funding_rate, max_abs_funding_rate]
    /// after applying the velocity. This prevents runaway rates from causing
    /// cascading liquidations and bad debt spirals during prolonged skew.
    ///
    /// Typical range: 0.50 to 1.00 (50% to 100% per day).
    pub max_abs_funding_rate: Udec,

    /// Maximum funding velocity, as a fraction per day.
    ///
    /// When skew == skew_scale, the funding rate changes by this much per day.
    /// When skew == 0, the rate drifts back toward zero at this speed.
    ///
    /// Typical range: 0.01 to 0.10 (1% to 10% per day).
    pub max_funding_velocity: Udec,

    /// Initial margin ratio for this pair.
    ///
    /// Determines the minimum collateral required to open a position.
    /// E.g., 0.05 = 5% = 20x maximum leverage.
    ///
    /// Required margin = position_size * price * initial_margin_ratio
    pub initial_margin_ratio: Udec,

    /// Maintenance margin ratio for this pair.
    ///
    /// Must be strictly less than `initial_margin_ratio`.
    ///
    /// When a user's equity falls below the sum of maintenance margins
    /// across all their positions, the user becomes eligible for liquidation.
    ///
    /// E.g., 0.025 = 2.5% → positions liquidated when effective leverage exceeds 40x.
    ///
    /// maintenance_margin = |position_size| * oracle_price * maintenance_margin_ratio
    pub maintenance_margin_ratio: Udec,

    /// Minimum notional value (in USD) for the opening portion of an order.
    /// Notional = |opening_size| * oracle_price.
    ///
    /// Only enforced on the opening portion — closing is always allowed.
    pub min_opening_notional: Udec,
}
```

#### State

Global state:

```rust
struct State {
    /// The vault's margin.
    ///
    /// This should equal the sum of all user deposits and the vault's realized PnL.
    ///
    /// Note that this doesn't equal the amount of funds withdrawable by burning
    /// shares (i.e. its "equity"), which also needs to factor in the vault's
    /// _unrealized_ PnL.
    pub vault_margin: Uint,

    /// Total supply of the vault's share token.
    pub vault_share_supply: Uint,
}
```

Pair-specific state:

```rust
struct PairState {
    /// The sum of the sizes of all long positions.
    ///
    /// E.g. suppose one futures contract represents 1 satoshi (1e-8 BTC).
    /// If `long_oi` has a value of 1,000,000, it means all long positions combined
    /// are 1,000,000 satoshis.
    ///
    /// Should always be non-negative.
    pub long_oi: Dec,

    /// The sum of the sizes of all short positions.
    ///
    /// Should always be non-positive.
    pub short_oi: Dec,

    /// Current instantaneous funding rate (fraction per day).
    ///
    /// Positive = longs pay shorts (and vault collects the net).
    /// Negative = shorts pay longs (and vault collects the net).
    ///
    /// The rate changes over time according to the velocity model:
    ///   rate' = rate + velocity * elapsed_days
    pub funding_rate: Dec,

    /// Timestamp of last funding accrual.
    pub last_funding_time: Timestamp,

    /// Cumulative funding per unit of position size, in settlement currency.
    ///
    /// This is an ever-increasing accumulator. To compute a position's accrued
    /// funding, take the difference between the current value and the position's
    /// `entry_funding_per_unit`.
    pub cumulative_funding_per_unit: Dec,

    /// Sum of `position.size * position.entry_funding_per_unit` across all open
    /// positions for this pair.
    ///
    /// Used to compute the vault's unrealized funding without iterating over
    /// all positions:
    ///   vault_unrealized_funding = oi_weighted_entry_funding - cumulative_funding_per_unit * skew
    pub oi_weighted_entry_funding: Dec,

    /// Sum of `position.size * position.entry_price` across all open
    /// positions for this pair.
    ///
    /// Used to compute the vault's unrealized price PnL without iterating over
    /// all positions:
    ///   vault_unrealized_pnl = oi_weighted_entry_price - oracle_price * skew
    pub oi_weighted_entry_price: Dec,
}
```

User-specific state:

```rust
struct UserState {
    // -------------------------------------------------------------------------
    // Counterparty Vault State
    // -------------------------------------------------------------------------

    /// The amount of vault shares this user owns.
    pub vault_shares: Uint,

    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: Vec<Unlock>,

    // -------------------------------------------------------------------------
    // Trading State
    // -------------------------------------------------------------------------

    /// Trading collateral balance in settlement currency (e.g., USDT).
    pub margin: Uint,

    /// Margin reserved for pending limit orders.
    ///
    /// When a limit order is placed but not immediately filled, the required
    /// margin is reserved (locked) to ensure the user can cover the position
    /// if/when the order executes.
    pub reserved_margin: Uint,

    /// Number of resting limit orders this user currently has on the book.
    /// Incremented when a limit order is stored, decremented on fill or cancel.
    pub open_order_count: u32,

    /// The user's open positions.
    pub positions: Map<PairId, Position>,
}

struct Position {
    /// Position size in contracts.
    ///
    /// Positive = long position (profits when price increases).
    /// Negative = short position (profits when price decreases).
    pub size: Dec,

    /// The average price at which this position was entered, in settlement
    /// currency per contract.
    ///
    /// Used for PnL calculation: PnL = size * (oracle_price - entry_price)
    ///
    /// Invariant across partial closes (only changes when adding to a
    /// position via weighted average), so no rounding error accumulates.
    pub entry_price: Udec,

    /// Value of `pair_state.cumulative_funding_per_unit` when this position
    /// was last opened, modified, or had funding settled.
    ///
    /// Used to compute accrued funding:
    ///   accrued = position.size * (current_cumulative - entry_funding_per_unit)
    ///
    /// Positive accrued funding means the trader owes the vault.
    pub entry_funding_per_unit: Dec,
}

struct Unlock {
    /// The amount of settlement currency to be released to the user once
    /// cooldown completes.
    pub amount_to_release: Uint,

    /// The time when cooldown completes.
    pub end_time: Timestamp,
}
```

#### Order

```rust
/// The Order struct stored as values in the indexed maps.
/// The key already encodes pair_id, limit_price, and created_at,
/// so the value only needs user_id, size, reduce_only, and reserved_margin.
struct Order {
    pub user_id: UserId,
    pub size: Dec,
    pub reduce_only: bool,
    pub reserved_margin: Uint,  // exact margin+fee reserved at placement
}

struct OrderIndexes {
    /// Index the orders by order ID, so that an order can be retrieve by:
    ///
    /// ```rust
    /// ORDERS.idx.order_id.load(order_id)
    /// ```
    pub order_id: UniqueIndex< /* ... */ >,

    /// Index the orders by user ID, so that we can retrieve all orders submitted
    /// by a user by:
    ///
    /// ```rust
    /// ORDERS.idx.user_id.prefix(user_id).range(...)
    /// ```
    pub user_id: MultiIndex< /* ... */ >,
}
```

### Storage layout

```rust
/// Global parameters.
const PARAMS: Item<Params> = Item::new("params");

/// Pair-specific parameters.
const PAIR_PARAMS: Map<PairId, PairParams> = Map::new("pair_params");

/// Global state.
const STATE: Item<State> = Item::new("state");

/// Pair-specific states.
const PAIR_STATES: Map<PairId, PairState> = Map::new("pair_state");

/// User states.
const USER_STATES: Map<UserId, UserState> = Map::new("user_state");

/// Buy orders indexed for descending price iteration (most competitive first).
/// Key: (pair_id, inverted_limit_price, created_at, order_id)
/// where inverted_limit_price = Udec::MAX - limit_price, so ascending iteration
/// yields descending prices.
const BIDS: IndexedMap<(PairId, Udec, Timestamp, OrderId), Order> = IndexedMap::new("bid", OrderIndexes {
    order_id: UniqueIndex::new(/* ... */),
    user_id: MultiIndex::new(/* ... */),
});

/// Sell orders indexed for ascending price iteration (more competitive first).
/// Key: (pair_id, limit_price, created_at, order_id)
const ASKS: IndexedMap<(PairId, Udec, Timestamp, OrderId), Order> = IndexedMap::new("ask", OrderIndexes {
    order_id: UniqueIndex::new(/* ... */),
    user_id: MultiIndex::new(/* ... */),
});
```

## Business logic

For simplicity, we assume the settlement asset (defined by `settlement_currency` in `Params`) is USDT.

### Margin deposit

Traders deposit settlement currency to their trading balance before opening positions:

```rust
fn handle_deposit_margin(
    user_state: &mut UserState,
    amount_received: Uint,
) {
    user_state.margin += amount_received;
}
```

### Margin withdrawal

Traders can withdraw available margin (funds not backing positions or reserved for orders):

```rust
fn handle_withdraw_margin(
    state: &mut State,
    user_state: &mut UserState,
    pair_states: &mut Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, Udec>,
    amount: Uint,
    current_time: Timestamp,
) {
    ensure!(amount > 0, "nothing to do");

    // Accrue funding for all pairs the user has positions in,
    // so that the equity calculation is up-to-date.
    for (pair_id, _) in &user_state.positions {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = &pair_params_map[&pair_id];
        let oracle_price = oracle_prices[&pair_id];
        accrue_funding(pair_state, pair_params, oracle_price, current_time);
    }

    ensure!(
      amount <= compute_available_margin(user_state, oracle_prices, pair_params_map, pair_states),
      "insufficient available margin"
    );

    user_state.margin -= amount;

    // Transfer `amount` of settlement currency to user.
}
```

Margin is the collateral required to open and maintain positions. We distinguish three types:

- **Used margin**: Collateral currently backing open positions
- **Reserved margin**: Collateral locked for pending limit orders (not yet filled)
- **Equity**: `margin + unrealized_pnl - accrued_funding` (the user's true economic balance)
- **Available margin**: `max(0, equity - used_margin - reserved_margin)`

```rust
/// Compute the margin available for withdrawal.
fn compute_available_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
) -> Uint {
    let equity = compute_user_equity(user_state, oracle_prices, pair_states);
    let used = compute_used_margin(user_state, oracle_prices, pair_params_map);

    // equity - used - reserved, floored at zero.
    // Equity is Dec (signed) since unrealized PnL can make it negative.
    let available = equity - used - user_state.reserved_margin;

    max(floor(available), 0)
}

/// Compute the margin currently used by open positions.
///
/// For each position, the used margin is:
///   |position_size| * current_price * initial_margin_ratio
fn compute_used_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
) -> Uint {
    let mut total = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        // Used margin = |size| * price * initial_margin_ratio
        let margin = abs(position.size) * oracle_price * pair_params.initial_margin_ratio;

        total += floor(margin);
    }

    total
}

/// Compute total used margin with one position projected to a new size.
///
/// Identical to `compute_used_margin`, but overrides the size for
/// `projected_pair_id` with `projected_size`. If the user has no existing
/// position in that pair, adds it as a new entry.
///
/// Used by the post-fill margin check (step 6 of order submission) to
/// validate that the user can cover the projected position before executing.
fn compute_projected_used_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    projected_pair_id: PairId,
    projected_size: Dec,
) -> Uint {
    let mut total = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        let size = if pair_id == projected_pair_id {
            projected_size
        } else {
            position.size
        };

        let margin = abs(size) * oracle_price * pair_params.initial_margin_ratio;

        total += floor(margin);
    }

    // If the projected pair is not among existing positions, add it.
    if !user_state.positions.contains_key(&projected_pair_id) && projected_size != Dec::ZERO {
        let oracle_price = oracle_prices[&projected_pair_id];
        let pair_params = pair_params_map[&projected_pair_id];
        let margin = abs(projected_size) * oracle_price * pair_params.initial_margin_ratio;

        total += floor(margin);
    }

    total
}

/// Compute the unrealized PnL for a position at the current oracle price.
///
/// unrealized_pnl = size * (oracle_price - entry_price)
///
/// Long  (size > 0): profits when oracle_price > entry_price. ✓
/// Short (size < 0): profits when oracle_price < entry_price. ✓
///
/// Positive = position is in profit. Negative = position is in loss.
fn compute_position_unrealized_pnl(
    position: &Position,
    oracle_price: Udec,
) -> Dec {
    position.size * (oracle_price - position.entry_price)
}

/// Compute the user's equity: margin balance adjusted for unrealized PnL
/// and accrued funding across all open positions.
///
/// equity = margin + Σ unrealized_pnl - Σ accrued_funding
///
/// Accrued funding sign convention: positive = trader owes vault (cost),
/// so subtracting it reduces equity when the trader owes.
///
/// NOTE: For maximum accuracy, `accrue_funding` should be called for each
/// pair before invoking this function, so that `cumulative_funding_per_unit`
/// is up-to-date.
fn compute_user_equity(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_states: &Map<PairId, PairState>,
) -> Dec {
    let mut total_unrealized_pnl = Dec::ZERO;
    let mut total_accrued_funding = Dec::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[&pair_id];
        let pair_state = &pair_states[&pair_id];

        total_unrealized_pnl += compute_position_unrealized_pnl(position, oracle_price);
        total_accrued_funding += compute_accrued_funding(position, pair_state);
    }

    // margin is Uint; convert to Dec for signed arithmetic
    user_state.margin + total_unrealized_pnl - total_accrued_funding
}
```

### Cancel order

Users can cancel pending limit orders to release their reserved margin:

```rust
fn handle_cancel_order(
    params: &Params,
    user_state: &mut UserState,
    pair_id: PairId,
    order_id: OrderId,
    pair_params: &PairParams,
) {
    // Find and remove the order from the appropriate order book
    let order = if let Some(order) = remove_buy_order(pair_id, order_id) {
        order
    } else if let Some(order) = remove_sell_order(pair_id, order_id) {
        order
    } else {
        ensure!(false, "order not found");
    };

    // Ensure the user owns this order
    ensure!(order.user_id == caller_user_id, "not your order");

    // Release the exact reserved margin stored in the order
    user_state.reserved_margin -= order.reserved_margin;
    user_state.open_order_count -= 1;
}

/// Compute the margin required for the opening portion of an order.
///
/// Only the opening portion (new exposure) requires margin.
/// The closing portion releases margin.
fn compute_required_margin(
    opening_size: Dec,
    limit_price: Udec,
    pair_params: &PairParams,
) -> Uint {
    if opening_size == Dec::ZERO {
        return Uint::ZERO;
    }

    let required_margin = abs(opening_size) * limit_price * pair_params.initial_margin_ratio;

    ceil(required_margin)  // Round up to be conservative (disadvantage to user)
}
```

### Submit order

#### Constraints

The main challenge for handling orders is it may not be possible to execute an order fully, due to a number of constraints:

1. **Max OI**: the long or short OI can't exceed a set maximum.
2. **Target price**: the worst acceptable execution price of the order, specified by the user.

An order may consists of two portions: one portion that reduces or closes an existing position (the "**closing portion**"); a portion that opens a new or increases an existing position (the "**opening portion**"). E.g. a user has a position of +50 contracts;

- an buy order of +100 contracts consists of -0 contract that reduces the existing position, and +100 contracts of increasing the current position;
- a sell order of -100 contracts consists of -50 contracts that reduces the existing long position, and -50 contracts of opening a new short position.

The closing portion is never subject to the OI constraint. The opening portion is subject to the OI constraint. The behavior when the opening portion would violate the OI constraint depends on the `reduce_only` flag:

- **`reduce_only = true`**: Only the closing portion is executed; the opening portion is discarded. This allows users to close or reduce positions even when OI is at its limit.
- **`reduce_only = false`**: The full order is attempted; if the opening portion would violate OI constraints, only the closing portion fills. The opening portion is all-or-nothing: it either fills fully or not at all.

The target price constraint (2) also uses **all-or-nothing** semantics: if the execution price of the fill would exceed the target price, the entire fill is rejected.

For each order, upon receiving it, we decompose it into closing and opening portions, check OI constraints on the opening portion, and then check if the execution price satisfies the target price. If a portion is left unfilled, it depends on the order kind: for market orders, the unfilled portion is canceled (immediate-or-cancel behavior); for limit orders, the unfilled portion is persisted in the contract storage until either it becomes fillable later as oracle price and OI change, or the user cancels it (good-til-canceled behavior).

#### Skew pricing

The pricing offered by the counterparty vault depends not only on the oracle price at the time, but also on the skew, and the order's size (i.e. how the order would change the skew).

**Skew** is defined as:

```rust
let skew = pair_state.long_oi + pair_state.short_oi;
```

Note that `short_oi` is non-positive.

- If `skew` is positive, it means all traders combined have a net long exposure, and the counterparty vault has a net short exposure. To incentivize traders to go back to neutral, the vault will offer better prices for selling, and worse price for buying.
- If `skew` is negative, it means all traders combined have a net short exposure, and the counterparty vault has a net long exposure. To incentivize traders to go back to neutral, the vault will offer better prices for buying, and worse price for selling.
- If `skew` is zero, it means the market is perfectly neutral, and the vault will not bias towards either buying or selling.

Given a skew and a size, the execution price is calculated as:

```rust
fn compute_exec_price(
    oracle_price: Udec,
    skew: Dec,
    size: Dec,
    pair_params: PairParams,
) -> Udec {
    // The skew after the size is fulfilled.
    let skew_after = skew + size;

    // The average skew before and after the size is fulfilled.
    let skew_average = (skew + skew_after) / 2;

    // Compute a premium based on the average skew and skew scaling factor.
    let premium = skew_average / pair_params.skew_scale;

    // Bound the premium between [-max_abs_premium, max_abs_premium].
    let premium = clamp(premium, -pair_params.max_abs_premium, pair_params.max_abs_premium);

    // Apply the premium to the oracle price to arrive at the final execution price.
    oracle_price * (1 + premium)
}

/// Marginal price is the execution price of an order of infinitesimal size.
/// This is equivalent to calling
///
/// ```rust
/// compute_exec_price(oracle_price, skew, 0, pair_params)
/// ```
///
/// but slightly optimized, since we know the size is zero.
fn compute_marginal_price(oracle_price: Udec, skew: Dec, pair_params: &PairParams) -> Udec {
    let premium = skew / pair_params.skew_scale;
    let premium = clamp(premium, -pair_params.max_abs_premium, pair_params.max_abs_premium);

    oracle_price * (1 + premium)
}

/// Bound the value `x` within the range `[min, max]`.
fn clamp(x: Dec, min: Dec, max: Dec) -> Dec {
    min(max(x, min), max)
}
```

#### Decompose order into closing vs opening

On receiving an order, we first decompose it into closing and opening portions.

```rust
/// Returns (closing_size, opening_size) where closing_size reduces existing
/// exposure and opening_size creates new exposure.
/// Both have the same sign as the original size (or are zero).
fn decompose_fill(size: Dec, user_pos: Dec) -> (Dec, Dec) {
    if size == 0 {
        return (0, 0);
    }

    // Closing occurs when size and position have opposite signs
    if size > 0 && user_pos < 0 {
        // Buy order, user has short position
        // Closing portion: min(size, |user_pos|)
        let closing = min(size, -user_pos);
        let opening = size - closing;
        (closing, opening)
    } else if size < 0 && user_pos > 0 {
        // Sell order, user has long position
        // Closing portion: max(size, -user_pos) [both negative]
        let closing = max(size, -user_pos);
        let opening = size - closing;
        (closing, opening)
    } else {
        // No closing: size and position have same sign (or position is zero)
        (0, size)
    }
}
```

#### Compute target price

We then compute the order's target price. The user can specify this in two ways:

- Market: the best available price in the market, plus/minus a maximum slippage. The best available price is also known as the **marginal price**, i.e. the price for executing an order of infinitesimal size.
- Limit: the user directly gives the price.

```rust
fn compute_target_price(
    kind: OrderKind,
    oracle_price: Udec,
    skew: Dec,
    pair_params: &PairParams,
    is_buy: bool,
) -> Udec {
    match kind {
        OrderKind::Market { max_slippage } => {
            // Marginal price is the execution price for an infinitesimal order
            let marginal_premium = clamp(
                skew / pair_params.skew_scale,
                -pair_params.max_abs_premium,
                pair_params.max_abs_premium
            );
            let marginal_price = oracle_price * (1 + marginal_premium);

            if is_buy {
                marginal_price * (1 + max_slippage)
            } else {
                marginal_price * (1 - max_slippage)
            }
        }
        OrderKind::Limit { limit_price } => limit_price,
    }
}

/// Check whether the execution price satisfies the price constraint.
///
/// - For buys:  exec_price <= target_price  (buyer won't pay more)
/// - For sells: exec_price >= target_price  (seller won't accept less)
fn check_price_constraint(exec_price: Udec, target_price: Udec, is_buy: bool) -> bool {
    if is_buy {
        exec_price <= target_price
    } else {
        exec_price >= target_price
    }
}
```

#### Max fillable from OI constraint

We then compute the maximum fillable amount based on the max OI constraint. This applies only to the opening portion of the order.

```rust
fn compute_max_opening_from_oi(
    opening_size: Dec,
    pair_state: &PairState,
    max_abs_oi: Udec,
) -> Dec {
    if opening_size > 0 {
        // Opening a long: increases long_oi
        let room = max_abs_oi - pair_state.long_oi;
        min(opening_size, room)
    } else if opening_size < 0 {
        // Opening a short: increases |short_oi|
        let room = max_abs_oi - pair_state.short_oi.abs();
        max(opening_size, -room)
    } else {
        0
    }
}

/// Determine the actual fill size after applying the open interest constraint.
///
/// The closing portion is never subject to OI limits. The opening portion is
/// all-or-nothing: if it exceeds `max_abs_oi`, only the closing portion fills
/// and the opening portion is rejected entirely.
///
/// Returns the fill size, which may be the full order or the closing portion
/// only.
fn compute_fill_size_from_oi(
    size: Dec,
    closing_size: Dec,
    opening_size: Dec,
    is_buy: bool,
    pair_state: &PairState,
    max_abs_oi: Udec,
) -> Dec {
    let max_opening = compute_max_opening_from_oi(opening_size, pair_state, max_abs_oi);

    let oi_violated = if is_buy {
        max_opening < opening_size
    } else {
        max_opening > opening_size
    };

    if oi_violated { closing_size } else { size }
}
```

#### Validate notional constraints

```rust
/// Validate minimum opening notional constraint.
/// If the order opens or increases exposure, the opening notional must meet
/// `min_opening_notional`. Closing portions are exempt so users can always exit.
fn validate_notional_constraints(
    opening_size: Dec,
    oracle_price: Udec,
    pair_params: &PairParams,
) {
    if opening_size != Dec::ZERO {
        // Opening or increasing a position: enforce minimum order notional.
        // Closing is exempt so users can always exit positions.
        let opening_notional = abs(opening_size) * oracle_price;
        ensure!(
            opening_notional >= pair_params.min_opening_notional,
            "opening notional below minimum"
        );
    }
}
```

#### Putting things together

```rust
fn handle_submit_order(
    params: &Params,
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    user_state: &mut UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    pair_id: PairId,
    oracle_price: Udec,
    size: Dec,
    kind: OrderKind,
    reduce_only: bool,
    current_time: Timestamp,
) {
    ensure!(size != 0, "nothing to do");

    // Step 1: Accrue funding before any OI changes
    accrue_funding(pair_state, pair_params, oracle_price, current_time);

    let skew = pair_state.long_oi + pair_state.short_oi;
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let is_buy = size > 0;

    // Step 2: Decompose into closing and opening portions.
    // If reduce_only, zero out the opening portion so the order can only
    // reduce existing exposure.
    let (closing_size, opening_size) = decompose_fill(size, user_pos);
    let opening_size = if reduce_only { Dec::ZERO } else { opening_size };

    // Step 3: Validate minimum opening notional
    validate_notional_constraints(opening_size, oracle_price, pair_params);

    // Step 4: Check OI constraint and determine fill size.
    // Use the reduce_only-adjusted effective size (closing + adjusted opening).
    let effective_size = closing_size + opening_size;
    let fill_size = compute_fill_size_from_oi(
        effective_size, closing_size, opening_size, is_buy,
        pair_state, pair_params.max_abs_oi,
    );
    ensure!(fill_size != Dec::ZERO, "order would have no effect");

    // Step 5: Post-fill margin check.
    // Project the new position size and compute total used margin (initial
    // margin ratio, oracle price). Ensure the user can cover the projected
    // position plus reserved margin after paying the trading fee.
    let exec_price = compute_exec_price(oracle_price, skew, fill_size, pair_params);
    let projected_size = user_pos + fill_size;
    let post_fill_used_margin = compute_projected_used_margin(
        user_state, oracle_prices, pair_params_map,
        pair_id, projected_size,
    );
    let trading_fee = compute_trading_fee(fill_size, exec_price, params.trading_fee_rate);
    let equity = compute_user_equity(user_state, oracle_prices, pair_states);
    ensure!(
        equity - trading_fee >= post_fill_used_margin + user_state.reserved_margin,
        "insufficient margin"
    );

    // Step 6: Compute execution price and check price constraint.
    // If the price fails: market orders error, limit orders go to the book.
    let target_price = compute_target_price(&kind, oracle_price, skew, pair_params, is_buy);

    if !check_price_constraint(exec_price, target_price, is_buy) {
        match kind {
            OrderKind::Market { .. } => {
                bail!("price exceeds slippage tolerance");
            },
            OrderKind::Limit { limit_price } => {
                // GTC: store the full original order and return.
                // No fill happened, so user_pos is unchanged.
                store_limit_order(
                    params, user_state, pair_params, pair_id, user_id,
                    size, limit_price, reduce_only,
                    user_pos, current_time,
                    oracle_prices, pair_params_map, pair_states,
                );

                return;
            },
        }
    }

    // Step 7: Execute fill and collect trading fee.
    execute_fill(state, pair_state, user_state, pair_id, fill_size, exec_price);
    collect_trading_fee(state, user_state, fill_size, exec_price, params.trading_fee_rate);
}

/// Store the unfilled portion of a limit order for later fulfillment (GTC).
///
/// Validates the user's open order count, reserves margin for the opening
/// portion of the unfilled size, and persists the order in the appropriate
/// side of the order book.
///
/// Buy orders are stored with inverted price (Udec::MAX - limit_price) so
/// ascending iteration yields descending price order.
fn store_limit_order(
    params: &Params,
    user_state: &mut UserState,
    pair_params: &PairParams,
    pair_id: PairId,
    user_id: UserId,
    unfilled_size: Dec,
    limit_price: Udec,
    reduce_only: bool,
    user_pos_after_fill: Dec,
    current_time: Timestamp,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
) {
    // Enforce maximum open orders
    ensure!(
        user_state.open_order_count < params.max_open_orders,
        "too many open orders"
    );

    // Reserve margin for the opening portion of the unfilled size.
    // With reduce_only, the opening portion is zero so no margin is reserved.
    let (_, unfilled_opening) = decompose_fill(unfilled_size, user_pos_after_fill);
    let unfilled_opening = if reduce_only { Dec::ZERO } else { unfilled_opening };
    let margin_to_reserve = compute_required_margin(unfilled_opening, limit_price, pair_params);
    let fee_to_reserve = compute_trading_fee(unfilled_opening, limit_price, params.trading_fee_rate);
    let reserved = margin_to_reserve + fee_to_reserve;

    // Check that the user has sufficient available margin to cover the reservation.
    let available_margin = compute_available_margin(user_state, oracle_prices, pair_params_map, pair_states);
    ensure!(available_margin >= reserved, "insufficient margin for limit order");

    user_state.reserved_margin += reserved;

    // Increment open order count
    user_state.open_order_count += 1;

    // Store limit order as GTC
    let order = Order {
        user_id,
        size: unfilled_size,
        reduce_only,
        reserved_margin: reserved,
    };
    let order_id = generate_order_id();

    if unfilled_size > Dec::ZERO {
        // Buy order: store with inverted price for descending iteration
        let key = (pair_id, Udec::MAX - limit_price, current_time, order_id);
        BIDS.save(key, order);
    } else {
        // Sell order: store with normal price for ascending iteration
        let key = (pair_id, limit_price, current_time, order_id);
        ASKS.save(key, order);
    }
}

/// Execute a fill, updating positions and settling PnL and funding for any
/// closing portion.
///
/// IMPORTANT: The caller must call `accrue_funding` before calling this
/// function. This ensures `cumulative_funding_per_unit` is up-to-date
/// before we settle per-position funding and update `oi_weighted_entry_funding`.
fn execute_fill(
    state: &mut State,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    pair_id: PairId,
    fill_size: Dec,
    exec_price: Udec,
) {
    let position = user_state.positions.get_mut(&pair_id);

    // Decompose into closing and opening portions
    let user_pos = position.map(|p| p.size).unwrap_or(Dec::ZERO);
    let (closing_size, opening_size) = decompose_fill(fill_size, user_pos);

    // Settle accrued funding and price PnL for existing position
    if let Some(pos) = position {
        // Remove old contribution to oi_weighted_entry_price BEFORE any
        // size modifications.
        pair_state.oi_weighted_entry_price -= pos.size * pos.entry_price;

        // Settle funding BEFORE modifying the position.
        // This also updates oi_weighted_entry_funding.
        settle_funding(state, pair_state, user_state, pos);

        // Settle price PnL for the closing portion
        if closing_size != Dec::ZERO {
            let pnl = compute_pnl_to_realize(pos, closing_size, exec_price);
            settle_pnl(state, user_state, pnl);
            // No entry_price update needed — it is invariant across partial closes.
        }
    }

    // Update position size and oi_weighted_entry_funding
    if let Some(pos) = position {
        // Remove old contribution to oi_weighted_entry_funding
        // (settle_funding already set entry to current cumulative, so this
        // removes the post-settlement contribution before we change the size)
        pair_state.oi_weighted_entry_funding -= pos.size * pos.entry_funding_per_unit;

        pos.size += fill_size;

        // Blend entry_price as weighted average for opening portion
        if opening_size != Dec::ZERO {
            let remaining_size = pos.size - opening_size;  // size after closing, before opening
            if remaining_size == Dec::ZERO {
                // Position was fully closed then reopened (opposite side)
                pos.entry_price = exec_price;
            } else {
                // Weighted average of remaining position and new opening
                pos.entry_price = (abs(remaining_size) * pos.entry_price + abs(opening_size) * exec_price)
                    / abs(pos.size);
            }
        }

        // Remove position if fully closed, or re-add contribution
        if pos.size == Dec::ZERO {
            user_state.positions.remove(&pair_id);
            // oi_weighted_entry_funding contribution already removed above
            // oi_weighted_entry_price contribution already removed in block above
        } else {
            // entry_funding_per_unit stays at current cumulative (set by settle_funding)
            pair_state.oi_weighted_entry_funding += pos.size * pos.entry_funding_per_unit;
            pair_state.oi_weighted_entry_price += pos.size * pos.entry_price;
        }
    } else if opening_size != Dec::ZERO {
        // Create new position
        let entry_funding = pair_state.cumulative_funding_per_unit;
        user_state.positions.insert(pair_id, Position {
            size: fill_size,
            entry_price: exec_price,
            entry_funding_per_unit: entry_funding,
        });

        // Add new contribution to oi_weighted_entry_funding
        pair_state.oi_weighted_entry_funding += fill_size * entry_funding;

        // Add new contribution to oi_weighted_entry_price
        pair_state.oi_weighted_entry_price += fill_size * exec_price;
    }

    // Update OI based on opening/closing portions
    // Opening portion increases OI on the respective side
    if opening_size > Dec::ZERO {
        pair_state.long_oi += opening_size;
    } else if opening_size < Dec::ZERO {
        pair_state.short_oi += opening_size;
    }

    // Closing portion decreases OI on the opposite side
    if closing_size > Dec::ZERO {
        // Buying to close a short: short_oi becomes less negative
        pair_state.short_oi += closing_size;
    } else if closing_size < Dec::ZERO {
        // Selling to close a long: long_oi decreases
        pair_state.long_oi += closing_size;
    }
}

/// Compute the PnL to be realized when closing a portion of a position.
///
/// Named `compute_pnl_to_realize` (not `compute_realized_pnl`) to distinguish
/// from PnL that was already realized in the past, which will be tracked
/// separately in UserState.
///
/// PnL = (exit_value - entry_value) for longs
/// PnL = (entry_value - exit_value) for shorts
///
/// Where:
/// - entry_value = |closing_size| * entry_price
/// - exit_value  = |closing_size| * exec_price
fn compute_pnl_to_realize(
    position: &Position,
    closing_size: Dec,  // Same sign as the order (positive for buys, negative for sells)
    exec_price: Udec,
) -> Dec {
    if closing_size == Dec::ZERO || position.size == Dec::ZERO {
        return Dec::ZERO;
    }

    let entry_value = abs(closing_size) * position.entry_price;
    let exit_value = abs(closing_size) * exec_price;

    // PnL direction depends on position direction
    if position.size > Dec::ZERO {
        // Long position: profit when exit > entry
        exit_value - entry_value
    } else {
        // Short position: profit when entry > exit
        entry_value - exit_value
    }
}

/// Settle realized PnL between user and vault.
///
/// - Positive PnL (user wins): vault pays user
/// - Negative PnL (user loses): user pays vault
fn settle_pnl(
    state: &mut State,
    user_state: &mut UserState,
    pnl: Dec,
) {
    if pnl > Dec::ZERO {
        // User wins: transfer from vault to user
        let amount = floor(pnl);
        state.vault_margin = state.vault_margin.saturating_sub(amount);
        user_state.margin += amount;
    } else if pnl < Dec::ZERO {
        // User loses: transfer from user to vault
        let loss = floor(-pnl);
        let user_pays = min(loss, user_state.margin);
        // Bad debt (loss - user_pays) is absorbed by the vault - they simply
        // don't receive payment for it. Proper liquidation should prevent this.
        user_state.margin -= user_pays;
        state.vault_margin += user_pays;
    }
    // pnl == 0: no transfer needed
}
```

### Trading fees

A trading fee is charged on every voluntary fill (market and limit orders) as a percentage of notional value. The fee is transferred from the user's margin to the vault (protocol revenue).

- Fee formula: `fee = ceil(|fill_size| * exec_price * trading_fee_rate)` — rounds up to advantage the protocol, consistent with the spec's rounding principle.
- If the user's margin is insufficient to cover the full fee, the actual fee is capped at `user_state.margin` (same pattern as `settle_pnl` and the liquidation fee).
- **Liquidation fills are exempt**: the existing `liquidation_fee_rate` (paid to the vault) is the only fee on liquidation. Charging both would be double-dipping. This matches Binance/OKX/dYdX/Drift where a dedicated liquidation fee replaces the normal trading fee.

```rust
/// Compute the trading fee for a given size and price.
fn compute_trading_fee(size: Dec, price: Udec, trading_fee_rate: Udec) -> Uint {
    ceil(abs(size) * price * trading_fee_rate)
}

fn collect_trading_fee(
    state: &mut State,
    user_state: &mut UserState,
    fill_size: Dec,
    exec_price: Udec,
    trading_fee_rate: Udec,
) -> Uint {
    let fee = compute_trading_fee(fill_size, exec_price, trading_fee_rate);
    let actual_fee = min(fee, user_state.margin);

    user_state.margin -= actual_fee;
    state.vault_margin += actual_fee;

    actual_fee
}
```

### Fulfillment of limit orders

At the beginning of each block, validators submit the latest oracle prices. The contract is then triggered to scan the unfilled limit orders in its storage and look for ones that can be filled.

For buy orders, we iterate from the orders with the highest limit price descendingly, until we reach `limit_price < marginal_price`. From this point beyond, no more buy orders can be filled.

For sell orders, we do the opposite: iterate from the lowest limit price ascendingly, until we reach `limit_price > marginal_price`.

Importantly, we execute both order types in an interleaving manner, which is necessary to ensure faireness. Suppose instead we execute all the buy orders first, then all the sell orders. The buy orders would increase the marginal price, allowing sell orders to execute at higher prices. This favors the sellers and disfavors the buyers. Instead, we go through each side of the order book simultaneously, and pick the order that was created earlier, respecting the **price-time priority**.

```rust
fn fulfill_limit_orders_for_pair(
    params: &Params,
    state: &mut State,
    pair_id: PairId,
    oracle_price: Udec,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    current_time: Timestamp,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
) {
    // Accrue funding before any OI changes
    accrue_funding(pair_state, pair_params, oracle_price, current_time);

    let mut skew = pair_state.long_oi + pair_state.short_oi;

    // Create iterators for both sides. Pre-fetch the first (most competitive)
    // order from each side instead of using peekable iterators.
    let mut bids = BIDS.prefix(pair_id).range(..);
    let mut asks = ASKS.prefix(pair_id).range(..);
    let mut next_bid = bids.next();
    let mut next_ask = asks.next();

    loop {
        // Recompute marginal price at current skew (changes after each fill)
        let marginal_price = compute_marginal_price(oracle_price, skew, pair_params);

        // Check if each side's best order passes the marginal price cutoff.
        // Buy: limit_price >= marginal_price (buyer willing to pay at least marginal).
        // Sell: limit_price <= marginal_price (seller willing to accept at most marginal).
        let bid_eligible = next_bid.as_ref().is_some_and(|(key, _)| {
            (Udec::MAX - key.inverted_limit_price) >= marginal_price
        });
        let ask_eligible = next_ask.as_ref().is_some_and(|(key, _)| {
            key.limit_price <= marginal_price
        });

        // Select which side to process based on price-time priority.
        let is_buy = match (bid_eligible, ask_eligible) {
            // Both sides eligible: pick the older order (buy wins ties)
            (true, true) => {
                let bid_ts = next_bid.as_ref().unwrap().0.created_at;
                let ask_ts = next_ask.as_ref().unwrap().0.created_at;
                bid_ts <= ask_ts
            },
            // Only the bid is eligible. Fill the bid.
            (true, false) => true,
            // Only the ask is eligible. Fill the ask.
            (false, true) => false,
            // Neither is eligible. Terminate the loop.
            (false, false) => break,
        };

        // Take the selected order and advance that side's iterator.
        let (key, order) = if is_buy {
            let entry = next_bid.take().unwrap();
            next_bid = bids.next();
            entry
        } else {
            let entry = next_ask.take().unwrap();
            next_ask = asks.next();
            entry
        };

        let fill_size = try_fill_limit_order(
            params, key, order, pair_id, is_buy,
            state, oracle_price, skew, pair_state, pair_params,
            oracle_prices, pair_params_map, pair_states,
        );

        skew += fill_size;
    }
}

/// Attempt to fill a single limit order. Returns the fill size, or zero if
/// the order was skipped.
///
/// This function only concerns the fill logic for a single order. Iteration,
/// order selection, and skew tracking are the caller's responsibility.
///
/// The caller has already verified that this order passes the marginal price
/// cutoff.
///
/// OI semantics: the opening portion is all-or-nothing. If OI is violated,
/// only the closing portion fills and the order is removed (not kept in book
/// with reduced size).
///
/// NOTE: Margin is checked at fill time. Between placement and fill the
/// user's equity can deteriorate (PnL on other positions, funding fees,
/// other fills). If the account can no longer support the order, it is
/// cancelled and its reserved margin is released.
fn try_fill_limit_order(
    params: &Params,
    key: OrderKey,
    order: Order,
    pair_id: PairId,
    is_buy: bool,
    state: &mut State,
    oracle_price: Udec,
    skew: Dec,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
) -> Dec {
    // Recover limit_price: buy orders use inverted storage
    let limit_price = if is_buy {
        Udec::MAX - key.inverted_limit_price
    } else {
        key.limit_price
    };
    let order_book = if is_buy { BIDS } else { ASKS };

    // Load user state (needed for position decomposition and OI check)
    let mut user_state = load_user_state(order.user_id);
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);

    // Step 2: Decompose closing/opening portions.
    let (closing_size, opening_size) = decompose_fill(order.size, user_pos);
    let opening_size = if order.reduce_only { Dec::ZERO } else { opening_size };

    // Step 4: OI check — determine fill size from OI constraint
    // (closing portion always allowed).
    let effective_size = closing_size + opening_size;
    let fill_size = compute_fill_size_from_oi(
        effective_size, closing_size, opening_size, is_buy,
        pair_state, pair_params.max_abs_oi,
    );

    // If fill_size is zero (pure opening order with OI violated), skip.
    // OI is transient — order may become fillable in a future cycle.
    if fill_size == Dec::ZERO {
        return Dec::ZERO;
    }

    // Step 5: Margin check — ensure the user can cover the projected position.
    // If violated, cancel — the account can no longer support this order.
    let exec_price = compute_exec_price(oracle_price, skew, fill_size, pair_params);
    let projected_size = user_pos + fill_size;
    let post_fill_used_margin = compute_projected_used_margin(
        user_state, oracle_prices, pair_params_map,
        pair_id, projected_size,
    );
    let trading_fee = compute_trading_fee(fill_size, exec_price, params.trading_fee_rate);
    let equity = compute_user_equity(user_state, oracle_prices, pair_states);

    if equity - trading_fee < post_fill_used_margin + user_state.reserved_margin - order.reserved_margin {
        // Cancel: remove order, release reserved margin, return zero.
        user_state.reserved_margin -= order.reserved_margin;
        user_state.open_order_count -= 1;
        order_book.remove(key);
        save_user_state(user_state);
        return Dec::ZERO;
    }

    // Step 6: Price check — if exec_price doesn't satisfy limit_price, skip.
    // Skew is transient — order may become fillable in a future cycle.
    if !check_price_constraint(exec_price, limit_price, is_buy) {
        return Dec::ZERO;
    }

    // Step 7: Execute fill (may be full order or closing-only).
    execute_fill(state, pair_state, &mut user_state, pair_id, fill_size, exec_price);
    collect_trading_fee(state, &mut user_state, fill_size, exec_price, params.trading_fee_rate);

    // Remove order after fill (all-or-nothing: no partial orders remain)
    user_state.reserved_margin -= order.reserved_margin;
    user_state.open_order_count -= 1;
    order_book.remove(key);

    save_user_state(user_state);

    fill_size
}
```

### Funding fee

Funding fees incentivize the market toward neutrality (equal long and short OI) by charging the majority side and paying the minority side. We use a **velocity-based** model (Synthetix V2/V3 style): the funding _rate_ changes over time at a velocity proportional to the current skew. This gives the rate "memory" -- even if the skew briefly touches zero, the rate persists, providing a stronger and more sustained incentive for rebalancing than a simple proportional model.

**Flow**: Bilateral. The majority side pays, the minority side receives, and the vault collects the net difference (`skew * delta_accumulator`). This means the vault profits from funding whenever the market is skewed (which is most of the time), regardless of which side is dominant.

#### Funding velocity

The funding velocity determines how quickly the funding rate changes. It is proportional to the current skew:

```rust
/// Compute the current funding velocity (rate of change of the funding rate).
///
/// velocity = (skew / skew_scale) * max_funding_velocity
///
/// The velocity has the same sign as the skew:
/// - Positive skew (net long) → positive velocity → rate increases → longs pay more
/// - Negative skew (net short) → negative velocity → rate decreases → shorts pay more
/// - Zero skew → zero velocity → rate stays constant (drifts toward 0 naturally
///   only when the rate overshoots past zero)
fn compute_funding_velocity(
    pair_state: &PairState,
    pair_params: &PairParams,
) -> Dec {
    let skew = pair_state.long_oi + pair_state.short_oi;
    (skew / pair_params.skew_scale) * pair_params.max_funding_velocity
}
```

#### Current funding rate

The funding rate evolves linearly between accruals:

```rust
/// Compute the current funding rate, accounting for time elapsed since
/// the last accrual.
///
/// current_rate = clamp(last_rate + velocity * elapsed_days,
///                      -max_abs_funding_rate, max_abs_funding_rate)
///
/// The rate is clamped to prevent runaway funding that could cause
/// cascading liquidations and bad debt spirals during prolonged skew.
fn compute_current_funding_rate(
    pair_state: &PairState,
    pair_params: &PairParams,
    current_time: Timestamp,
) -> Dec {
    let elapsed_secs = current_time - pair_state.last_funding_time;
    let elapsed_days = elapsed_secs / 86400;

    let velocity = compute_funding_velocity(pair_state, pair_params);
    let unclamped = pair_state.funding_rate + velocity * elapsed_days;

    clamp(unclamped, -pair_params.max_abs_funding_rate, pair_params.max_abs_funding_rate)
}
```

#### Unrecorded funding per unit

Between accruals, funding accumulates but hasn't yet been recorded in `cumulative_funding_per_unit`. We compute this unrecorded portion using trapezoidal integration (the rate changes linearly, so the integral is exact):

```rust
/// Compute the funding per unit of position size that has accrued since
/// the last accrual but not yet been recorded, along with the current
/// funding rate.
///
/// Returns `(unrecorded_funding_per_unit, current_rate)`.
///
/// Uses trapezoidal integration: the rate changes linearly from
/// `last_rate` to `current_rate` over the elapsed period, so the
/// average rate is their midpoint.
///
/// unrecorded = avg_rate * elapsed_days * oracle_price
///
/// The oracle_price converts from "fraction of position size per day"
/// to settlement currency per contract.
///
/// The current rate is returned alongside the unrecorded funding to
/// avoid redundant computation (the rate is already needed for the
/// trapezoidal integration).
fn compute_unrecorded_funding_per_unit(
    pair_state: &PairState,
    pair_params: &PairParams,
    oracle_price: Udec,
    current_time: Timestamp,
) -> (Dec, Dec) {
    let elapsed_secs = current_time - pair_state.last_funding_time;
    let elapsed_days = elapsed_secs / 86400;

    let current_rate = compute_current_funding_rate(pair_state, pair_params, current_time);
    let avg_rate = (pair_state.funding_rate + current_rate) / 2;

    (avg_rate * elapsed_days * oracle_price, current_rate)
}
```

#### Accruing funding (global)

This function updates the global pair state to reflect accumulated funding. It **must** be called before any operation that changes OI (fills, liquidations) to ensure the accumulator is up-to-date.

```rust
/// Accrue funding for a pair: update the cumulative accumulator,
/// the current rate, and the timestamp.
///
/// MUST be called before any OI-changing operation (execute_fill,
/// liquidation) to ensure correct accounting.
fn accrue_funding(
    pair_state: &mut PairState,
    pair_params: &PairParams,
    oracle_price: Udec,
    current_time: Timestamp,
) {
    let (unrecorded, current_rate) = compute_unrecorded_funding_per_unit(
        pair_state,
        pair_params,
        oracle_price,
        current_time,
    );

    // Update the funding rate, funding accumulator, and timestamp.
    pair_state.funding_rate = current_rate;
    pair_state.last_funding_time = current_time;
    pair_state.cumulative_funding_per_unit += unrecorded;
}
```

#### Per-position accrued funding

```rust
/// Compute the funding accrued by a specific position since it was
/// last touched (opened, modified, or had funding settled).
///
/// accrued = position.size * (current_cumulative - entry_cumulative)
///
/// Sign convention:
/// - Positive result = trader owes vault (cost to the trader)
/// - Negative result = vault owes trader (credit to the trader)
///
/// This follows from:
/// - When rate > 0: longs pay (size > 0 produces positive accrued)
/// - When rate < 0: shorts pay (size < 0, delta < 0, product is positive)
fn compute_accrued_funding(
    position: &Position,
    pair_state: &PairState,
) -> Dec {
    let delta = pair_state.cumulative_funding_per_unit - position.entry_funding_per_unit;
    position.size * delta
}
```

#### Settling funding for a position

When a position is touched (opened, modified, closed, or liquidated), its accrued funding is settled -- transferred between the trader and the vault -- and the position's entry point is reset.

```rust
/// Settle accrued funding for a position. Transfers funds between the
/// trader's margin and the vault.
///
/// This function:
/// 1. Computes the accrued funding since the position was last touched.
/// 2. Transfers the amount (positive = user pays vault, negative = vault pays user).
/// 3. Updates oi_weighted_entry_funding to remove the old contribution.
/// 4. Resets the position's entry_funding_per_unit to the current cumulative value.
/// 5. Updates oi_weighted_entry_funding to add the new contribution.
///
/// Steps 3-5 maintain the invariant:
///   oi_weighted_entry_funding = Σ (pos.size * pos.entry_funding_per_unit)
fn settle_funding(
    state: &mut State,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    position: &mut Position,
) {
    let accrued = compute_accrued_funding(position, pair_state);

    // Transfer funding between user and vault.
    // Positive accrued = user pays vault. Negative = vault pays user.
    // We reuse settle_pnl with negated sign (accrued funding is a cost to the
    // trader, so it's negative PnL from their perspective).
    settle_pnl(state, user_state, -accrued);

    // Update the oi_weighted_entry_funding accumulator:
    // Remove old contribution, update entry point, add new contribution.
    pair_state.oi_weighted_entry_funding -= position.size * position.entry_funding_per_unit;
    position.entry_funding_per_unit = pair_state.cumulative_funding_per_unit;
    pair_state.oi_weighted_entry_funding += position.size * position.entry_funding_per_unit;
}
```

### Liquidation and deleveraging

When a user's equity drops below their total maintenance margin, any third party ("liquidator") may call `handle_force_close` to liquidate the user. All positions are closed at skew-adjusted prices, pending limit orders are cancelled, and a fee proportional to the total notional value of the liquidated positions is transferred to the vault.

#### Maintenance margin

The maintenance margin mirrors `compute_used_margin` but uses the lower `maintenance_margin_ratio` instead of `initial_margin_ratio`:

```rust
/// Compute the total maintenance margin across all open positions.
///
/// Maintenance margin uses the same oracle-price basis as `compute_used_margin`
/// and `compute_user_equity` for consistent comparison.
fn compute_maintenance_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
) -> Uint {
    let mut total = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        // Maintenance margin = |size| * oracle_price * maintenance_margin_ratio
        let margin = abs(position.size) * oracle_price * pair_params.maintenance_margin_ratio;

        total += ceil(margin);  // Round up (conservative, disadvantage to user)
    }

    total
}
```

#### Liquidation check

```rust
/// Returns true if the user is eligible for liquidation.
///
/// A user is liquidatable when their equity (margin + unrealized PnL - accrued
/// funding) falls below their total maintenance margin.
///
/// NOTE: `accrue_funding` should be called for each pair before invoking this
/// function, so that `cumulative_funding_per_unit` is up-to-date.
fn is_liquidatable(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
) -> bool {
    // No positions means nothing to liquidate.
    if user_state.positions.is_empty() {
        return false;
    }

    let equity = compute_user_equity(user_state, oracle_prices, pair_states);
    let maintenance_margin = compute_maintenance_margin(user_state, oracle_prices, pair_params_map);

    equity < maintenance_margin
}
```

#### Cancel all orders

During liquidation, all pending limit orders must be cancelled before assessing solvency. This removes future exposure and releases reserved margin.

```rust
/// Cancel all pending limit orders for a user across all pairs.
///
/// Implementation note: requires a secondary index on `user_id` in both
/// BIDS and ASKS to efficiently find all orders belonging to a user.
fn cancel_all_orders(user_state: &mut UserState, user_id: UserId) {
    // Remove all bid orders for this user
    for (key, _order) in BIDS.idx.user_id.prefix(user_id) {
        BIDS.remove(key);
    }

    // Remove all ask orders for this user
    for (key, _order) in ASKS.idx.user_id.prefix(user_id) {
        ASKS.remove(key);
    }

    // All reserved margin is released; all orders are gone.
    user_state.reserved_margin = 0;
    user_state.open_order_count = 0;
}
```

#### Force close (liquidation handler)

```rust
/// Liquidate a user by closing all positions and paying a fee to the vault.
///
/// Can be called by any third party when the user is below maintenance margin.
/// Also callable by the admin for auto-deleveraging (see note below).
fn handle_force_close(
    params: &Params,
    state: &mut State,
    pair_states: &mut Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, Udec>,
    user_state: &mut UserState,
    user_id: UserId,
    current_time: Timestamp,
) {
    // Step 1: Cancel all pending limit orders.
    // This removes future exposure and releases reserved margin, giving the
    // user the best chance of meeting maintenance margin without liquidation.
    cancel_all_orders(user_state, user_id);

    // Step 2: Accrue funding for all pairs the user has positions in.
    // Must happen before equity/margin checks and before execute_fill.
    for (pair_id, _position) in &user_state.positions {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = pair_params_map[&pair_id];
        let oracle_price = oracle_prices[&pair_id];

        accrue_funding(pair_state, &pair_params, oracle_price, current_time);
    }

    // Step 3: Check liquidation condition.
    // After cancelling orders and accruing funding, the user may no longer
    // be underwater. Revert if the user is solvent.
    ensure!(
        is_liquidatable(user_state, oracle_prices, pair_params_map, pair_states),
        "user is not liquidatable",
    );

    // Step 4: Compute total notional for liquidation fee calculation.
    let mut total_notional = Udec::ZERO;
    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[&pair_id];
        total_notional += abs(position.size) * oracle_price;
    }

    // Step 5: Close all positions at skew-adjusted prices.
    //
    // Each position is closed by filling the opposite size. We use
    // skew-adjusted execution prices (not oracle) so that liquidation
    // pricing is identical to voluntary trading. This eliminates a moral
    // hazard where users might prefer being liquidated over self-closing
    // when the skew premium makes voluntary closure more expensive.
    //
    // The liquidation _trigger_ (Step 3) uses oracle-based equity, which
    // is safe from skew manipulation. Only execution uses skew-adjusted
    // prices, consistent with all other fills in the system.
    //
    // For liquidation, `decompose_fill` always yields 100% closing / 0%
    // opening, so OI constraints (max_abs_oi) don't apply.
    //
    // We collect `pair_ids` first to avoid borrowing conflicts.
    let pair_ids: Vec<PairId> = user_state.positions.keys().cloned().collect();

    for pair_id in pair_ids {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = pair_params_map[&pair_id];
        let oracle_price = oracle_prices[&pair_id];
        let position_size = user_state.positions[&pair_id].size;

        // Close by filling the exact opposite of the current position.
        let fill_size = -position_size;
        let skew = pair_state.long_oi + pair_state.short_oi;
        let exec_price = compute_exec_price(oracle_price, skew, fill_size, pair_params);

        execute_fill(state, pair_state, user_state, pair_id, fill_size, exec_price);
    }

    // Step 6: Pay liquidation fee to the vault.
    //
    // The fee is proportional to total notional, capped at the user's
    // remaining margin. Using notional (not remaining margin) ensures the
    // fee stays proportional to position risk — a margin-based fee
    // would shrink as the user deteriorates.
    //
    // Paying the fee to the vault (not the liquidator) prevents
    // self-liquidation exploits where a user force-closes themselves
    // from a second account to avoid trading fees.
    //
    // After all positions are closed, user_state.margin reflects the
    // user's remaining balance (could be zero if they had bad debt).
    let fee = floor(total_notional * params.liquidation_fee_rate);
    let actual_fee = min(fee, user_state.margin);

    user_state.margin -= actual_fee;
    state.vault_margin += actual_fee;
}
```

#### Price usage summary

The table below summarizes which price and margin ratio are used in each context:

| Context                       | Margin ratio               | Price used                                                            | Rationale                                                                                                   |
| ----------------------------- | -------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| New order margin requirement  | `initial_margin_ratio`     | Oracle price (for used margin projection) / Execution price (for fee) | Post-fill check: projects the new position at oracle price, computes fee at execution price                 |
| Existing position used margin | `initial_margin_ratio`     | Oracle price                                                          | Stable valuation for available margin; prevents over-leveraging on new orders/withdrawals                   |
| Liquidation trigger           | `maintenance_margin_ratio` | Oracle price                                                          | Must match equity basis (also oracle-based) for consistent comparison; immune to skew manipulation          |
| Liquidation execution         | N/A                        | Skew-adjusted price                                                   | Consistent with voluntary trading; eliminates moral hazard where users prefer liquidation over self-closing |

#### Auto-deleveraging

Auto-deleveraging (ADL) is a mechanism for the exchange to forcibly close positions of profitable traders when the counterparty vault cannot cover its obligations. This is left as future work. The `handle_force_close` function already supports admin-only invocation for this purpose — the admin would call it on selected users without requiring the maintenance margin check.

### Counterparty vault

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT coins to receive newly minted shares, or burn shares to redeem USDT. The amount of shares to be minted is determined by the total supply shares and the vault's current **equity**.

#### Vault equity

The vault's equity is defined as the vault's token balance (which reflects the total amount of deposit from liquidity providers and the vault's realized PnL) plus its unrealized PnL and unrealized funding:

```rust
fn compute_vault_equity(
    state: &State,
    pair_states: &Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, Udec>,
    usdt_price: Udec,
    current_time: Timestamp,
) -> Dec {
    let unrealized_pnl = compute_vault_unrealized_pnl(pair_states, oracle_prices);

    let unrealized_funding = compute_vault_unrealized_funding(
        pair_states,
        pair_params_map,
        oracle_prices,
        current_time,
    );

    state.vault_margin + ((unrealized_pnl + unrealized_funding) / usdt_price)
}

fn compute_vault_unrealized_pnl(
    pair_states: &Map<PairId, PairState>,
    oracle_prices: &Map<PairId, Udec>,
) -> Dec {
    let mut total = Dec::ZERO;

    for (pair_id, pair_state) in pair_states {
        let oracle_price = oracle_prices[&pair_id];
        let skew = pair_state.long_oi + pair_state.short_oi;

        // vault_pnl = -(total_trader_pnl)
        //           = -(oracle_price * skew - oi_weighted_entry_price)
        //           = oi_weighted_entry_price - oracle_price * skew
        total += pair_state.oi_weighted_entry_price - oracle_price * skew;
    }

    total
}

fn compute_vault_unrealized_funding(
    pair_states: &Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, Udec>,
    usdt_price: Udec,
    current_time: Timestamp,
) -> Dec {
    let mut unrealized_funding = Dec::ZERO;

    for (pair_id, pair_state) in pair_states {
        let oracle_price = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        unrealized_funding += compute_vault_unrealized_funding_for_pair(
            pair_state,
            pair_params,
            oracle_price,
            current_time,
        );
    }

    unrealized_funding
}

/// Compute the vault's unrealized funding for a single pair.
///
/// The total accrued funding across all positions is:
///   Σ pos.size * (cumulative - pos.entry_funding_per_unit)
///   = cumulative * Σ pos.size - Σ (pos.size * pos.entry_funding_per_unit)
///   = cumulative * skew - oi_weighted_entry_funding
///
/// The vault receives the opposite of what traders owe, so:
///   vault_unrealized_funding = oi_weighted_entry_funding - cumulative * skew
///
/// Additionally, we must include funding that has accrued since the last
/// `accrue_funding` call (the "unrecorded" portion).
///
/// Positive result = vault is owed money. Negative = vault owes traders.
fn compute_vault_unrealized_funding_for_pair(
    pair_state: &PairState,
    pair_params: &PairParams,
    oracle_price: Udec,
    current_time: Timestamp,
) -> Dec {
    let skew = pair_state.long_oi + pair_state.short_oi;

    // Funding already recorded in the accumulator
    let recorded = pair_state.oi_weighted_entry_funding
        - pair_state.cumulative_funding_per_unit * skew;

    // Funding accrued since last accrue_funding call
    let (unrecorded_per_unit, _) = compute_unrecorded_funding_per_unit(
        pair_state,
        pair_params,
        oracle_price,
        current_time,
    );
    let unrecorded = -skew * unrecorded_per_unit;

    recorded + unrecorded
}
```

#### Handling deposit

We use the following constant parameters to prevent the [ERC-4626 frontrunning donation attack](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v5.5.0/contracts/token/ERC20/extensions/ERC4626.sol#L22-L48). See the link for details.

```rust
/// Virtual shares added to total supply in share price calculations.
/// Prevents the first-depositor attack (ERC-4626 inflation attack) by
/// ensuring the share price cannot be trivially inflated.
const VIRTUAL_SHARES: Uint = 1_000_000;

/// Virtual assets added to vault equity in share price calculations.
/// Works in tandem with VIRTUAL_SHARES to set the initial share price
/// and prevent share inflation attacks.
const VIRTUAL_ASSETS: Uint = 1;
```

Suppose the vault receives `amount_received` units of USDT from user:

```rust
fn handle_deposit_liquidity(
    state: &mut State,
    user_state: &mut UserState,
    amount_received: Uint,
    min_shares_to_mint: Option<Uint>,
    usdt_price: Udec,
) {
    ensure!(amount_received > 0, "nothing to do");

    // Use virtual offsets to prevent the first-depositor share inflation attack.
    // By adding VIRTUAL_SHARES to supply and VIRTUAL_ASSETS to equity, the
    // share price cannot be trivially manipulated by donating to the vault.
    //
    // When the vault is empty (supply=0, equity=0):
    //   shares = amount * VIRTUAL_SHARES / VIRTUAL_ASSETS
    //          = amount * 1_000_000    (same scaling as before)
    //
    // When the vault is non-empty, the virtual offsets are negligible relative
    // to real supply/equity, so pricing is effectively unchanged.
    let effective_supply = state.vault_share_supply + VIRTUAL_SHARES;
    let effective_equity = compute_vault_equity(state, usdt_price) + VIRTUAL_ASSETS;

    ensure!(
        effective_equity > 0,
        "vault is in catastrophic loss! deposit disabled"
    );

    let shares_to_mint = floor(amount_received * effective_supply / effective_equity);

    // Ensure the number of shares to mint is no less than the minimum.
    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares would be minted"
        );
    }

    // Update global state.
    state.vault_margin += amount_received;
    state.vault_share_supply += shares_to_mint;

    // Update user state.
    user_state.vault_shares += shares_to_mint;
}
```

#### Handling withdrawal

Suppose user requests to burn `shares_to_burn` units of shares:

```rust
fn handle_unlock_liquidity(
    state: &mut State,
    user_state: &mut UserState,
    shares_to_burn: Uint,
    usdt_price: Udec,
    current_time: Timestamp,
) {
    ensure!(shares_to_burn > 0, "nothing to do");
    ensure!(user_state.vault_shares >= shares_to_burn, "can't burn more than what you have");

    // Similarly to deposit, first compute the vault's equity.
    let vault_equity = compute_vault_equity(state, usdt_price);

    ensure!(
        vault_equity > 0,
        "vault is in catastrophic loss! withdrawal disabled"
        // If equity is zero or negative, shares currently have no redeemable
        // value. Halting withdrawals (rather than burning shares for 0) preserves
        // the LP's claim in case the vault recovers (e.g. losing traders get
        // liquidated or prices revert). This mirrors the deposit-side check.
    );

    // Use the same virtual offsets as in deposit for symmetry.
    // Again, note the direction of rounding.
    let effective_supply = state.vault_share_supply + VIRTUAL_SHARES;
    let effective_equity = vault_equity + VIRTUAL_ASSETS;
    let amount_to_release = floor(effective_equity * shares_to_burn / effective_supply);

    ensure!(
        state.vault_margin >= amount_to_release,
        "the vault doesn't have sufficient balance to fulfill with this withdrawal"
        // This can happen if the vault has a very positive unrealized PnL.
        // In this case, the liquidity provider must wait until that PnL is realized
        // (i.e. the losing positions from traders are either closed or liquidated)
        // before withdrawing.
    );

    // Update global state.
    state.vault_margin -= amount_to_release;
    state.vault_share_supply -= shares_to_burn;

    // Update user state.
    user_state.vault_shares -= shares_to_burn;

    // Insert the new unlock into the user's state.
    let end_time = current_time + params.vault_cooldown_period;
    user_state.unlocks.push(Unlock { amount_to_release, end_time });
}
```

Once the cooldown period elapses, the contract needs to be triggered to release the fund and remove this unlock. This is trivial and we ignore it in this spec.

## Out-of-scope features

The following are out-of-scope for now, but will be added in the future:

v1.5 (to be shipped in relatively near future):

- **Isolated margin**
- **TP/SL**: automatically close the position if PnL reaches an upper or lower threshold.

v2 (to be shipped in further future):

- **Partial fill of orders**: see [v2.md](./v2.md).
- **Partial liquidation**: rather than closing all positions at once, reduce positions incrementally (e.g. close the largest position, or reduce all positions by a fixed percentage) until the user is back above maintenance margin. Reduces market impact and preserves user capital. Used by most major exchanges (Binance, Bybit, Hyperliquid, OKX, Drift).
