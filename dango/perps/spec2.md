# Perpetual futures exchange: specifications (v2 — order book model)

- This is a perpetual futures (perps) exchange that uses an **order book model** (CLOB — central limit order book), similar to e.g. Binance Futures, Hyperliquid, and dYdX. Users submit limit and market orders that are matched against each other using **price-time priority**. A protocol-managed **counterparty vault** acts as a regular market maker on the order book, providing liquidity via an on-chain requoting policy triggered on each oracle update. This is in contrast to the peer-to-pool model (spec.md) where the pool is the sole counterparty to all trades.
- The exchange operates in **one-way mode**. A user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, we modify the existing position instead of creating a new one.
- A **separated insurance fund** serves as the clearinghouse: all PnL settlement flows between users and the insurance fund (not directly between users). The vault deposits capital into the insurance fund via the same mechanism. This separation keeps LP claims clean and makes vault equity computable identically to any user.
- Funding fees incentivize the market toward neutrality (equal long and short OI), using the same velocity-based model as spec.md.
- For this v2 release:
  - Only **cross margin** is supported. Support for isolated margin may be added in a future update.
  - Orders support **partial fills**: if an incoming order cannot be fully matched, the unmatched remainder either cancels (market/IOC) or rests in the book (limit/GTC).

This spec is divided into three sections:

- _API_ defines the methods available for users to interact with the smart contract.
- _Storage_ defines what data are to be saved in the smart contract's storage.
- _Business logic_ defines how the smart contract should handle user requests through the API.

Regarding code snippets:

- Written in Rust-like pseudocode.
- We use the following typed number wrappers (defined in `dango/types/src/units.rs`) to make the dimensional meaning of each value explicit:
  - `BaseAmount` — an unsigned integer quantity of an asset in its _base unit_ (e.g. 1234 = 1234 uusdc). Used for settlement currency amounts and vault shares.
  - `HumanAmount` — a signed fixed-point decimal quantity in _human units_ (e.g. +1.234 BTC). Used for position sizes, order sizes, and open interest. Positive = long, negative = short.
  - `UsdValue` — a signed fixed-point decimal value in USD (e.g. $1.234). Used for notional values, PnL, and USD-denominated amounts.
  - `UsdPrice` — alias for `Ratio<UsdValue, HumanAmount>`, the price of an asset in USD per human unit.
  - `Ratio<N, D>` — a dimensionless or typed ratio between numerator type `N` and denominator type `D`. When `N == D` (written `Ratio<T>`), it is a dimensionless fraction in `T`-space. Examples: `Ratio<UsdValue>` for fee rates, `Ratio<UsdPrice>` for price-space fractions like slippage.
- In the pseudocode we ignore low-level conversion mechanics between these number types, except for when converting from decimal to integer, where it's necessary to specify whether it should be floored or ceiled.
- We also assume all the math traits are implemented. E.g. we may directly multiply a `HumanAmount` with a `UsdPrice` to obtain a `UsdValue`.

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
        min_shares_to_mint: Option<BaseAmount>,
    },

    /// Request to withdraw funds from the counterparty vault.
    ///
    /// The request will be fulfilled after the cooldown period, defined in the
    /// global parameters.
    UnlockLiquidity {
        /// The amount of vault shares the user wishes to burn.
        /// Must be no more than the amount of vault shares the user owns.
        shares_to_burn: BaseAmount,
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
        amount: BaseAmount,
    },

    /// Submit an order through the matching engine.
    ///
    /// The order is first matched against the opposite side of the book.
    /// Any unmatched remainder is either canceled (market/IOC) or placed
    /// on the book (limit/GTC).
    SubmitOrder {
        // The pair ID can either be numerical or string-like.
        pair_id: PairId,

        /// The amount of the futures contract to buy or sell.
        ///
        /// Positive for buy (increase long exposure, decrease short exposure);
        /// negative for sell (decrease long exposure, increase short exposure).
        size: HumanAmount,

        /// The order type: market, limit, etc.
        kind: OrderKind,

        /// If true, only the closing portion of the order is executed; any
        /// opening portion is discarded.
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
    /// Callable by anyone when the user is below maintenance margin.
    /// Positions are closed via market orders through the matching engine.
    /// If the book cannot fully absorb a liquidation, the vault backstops
    /// the remaining position at oracle price.
    Liquidate {
        /// The user's identifier. In the smart contract implementation, this
        /// should be an account address.
        user: UserId,
    },

    /// Auto-deleverage a user's position in a single pair.
    ///
    /// Triggered when a liquidation produces bad debt that exceeds the
    /// insurance fund balance. The most profitable opposing position is
    /// forcibly closed at the bankruptcy price.
    Deleverage {
        /// The user whose position to close.
        user: UserId,
        /// The pair to close the position in.
        pair_id: PairId,
    },
}

enum OrderKind {
    /// Trade at the best available prices in the order book, optionally
    /// with a slippage tolerance relative to the oracle price.
    ///
    /// If the order cannot be fully filled, the unfilled portion is
    /// canceled (immediate-or-cancel behavior).
    Market {
        /// The execution price must not be worse than the oracle price plus
        /// this slippage.
        ///
        /// For bids / buy orders:
        ///
        /// ```plain
        /// worst_fill_price <= oracle_price * (1 + max_slippage)
        /// ```
        ///
        /// For asks / sell orders:
        ///
        /// ```plain
        /// worst_fill_price >= oracle_price * (1 - max_slippage)
        /// ```
        max_slippage: Ratio<UsdPrice>,
    },

    /// Trade at the specified limit price.
    ///
    /// The order walks the book at or better than the limit price.
    /// Any unfilled remainder is persisted in the book as a resting
    /// order (good-til-canceled behavior).
    Limit {
        /// The execution price must be equal to or better than this price.
        limit_price: UsdPrice,
    },
}
```

Note: There is no `VaultSubmitOrder` or `VaultCancelOrder` in the API. The vault's orders are managed automatically by the on-chain requoting policy triggered by `OnOracleUpdate`.

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

    /// Trading fee charged to the maker (resting order) as a fraction of
    /// notional value.
    ///
    /// fee = |fill_size| * fill_price * maker_fee_rate
    ///
    /// Deducted from the maker's margin and transferred to the insurance fund.
    /// E.g., 0.0002 = 0.02%.
    pub maker_fee_rate: Ratio<UsdValue>,

    /// Trading fee charged to the taker (incoming order) as a fraction of
    /// notional value.
    ///
    /// fee = |fill_size| * fill_price * taker_fee_rate
    ///
    /// Deducted from the taker's margin and transferred to the insurance fund.
    /// E.g., 0.0005 = 0.05%.
    pub taker_fee_rate: Ratio<UsdValue>,

    /// Fee paid to the insurance fund as a fraction of the total notional value
    /// of positions being liquidated.
    ///
    /// E.g., 0.0005 = 0.05%. For $100,000 total notional, fee = $50.
    ///
    /// Capped at the user's remaining margin after position closure.
    pub liquidation_fee_rate: Ratio<UsdValue>,
}
```

Each trading pair is also associated with a set of pair-specific parameters:

```rust
struct PairParams {
    /// A scaling factor that determines how greatly an imbalance in open
    /// interest ("skew") should affect the funding rate velocity.
    /// The greater the value of the scaling factor, the less the effect.
    ///
    /// Used only for the funding fee mechanism (not for pricing, which is
    /// determined by the order book).
    pub skew_scale: Ratio<HumanAmount, Ratio<UsdPrice>>,

    /// The maximum allowed open interest for both long and short.
    /// I.e. the following must be satisfied:
    ///
    /// - pair_state.long_oi <= pair_params.max_abs_oi
    /// - pair_state.short_oi <= pair_params.max_abs_oi
    ///
    /// This constraint does not apply to reducing positions.
    pub max_abs_oi: HumanAmount,

    /// Maximum absolute funding rate, as a fraction per day.
    ///
    /// The funding rate is clamped to [-max_abs_funding_rate, max_abs_funding_rate]
    /// after applying the velocity.
    pub max_abs_funding_rate: Ratio<UsdValue, Duration>,

    /// Maximum funding velocity, as a fraction per day.
    ///
    /// When skew == skew_scale, the funding rate changes by this much per day.
    pub max_funding_velocity: Ratio<Ratio<UsdValue, Duration>, Duration>,

    /// Initial margin ratio for this pair.
    ///
    /// Determines the minimum collateral required to open a position.
    /// E.g., 0.05 = 5% = 20x maximum leverage.
    pub initial_margin_ratio: Ratio<UsdValue>,

    /// Maintenance margin ratio for this pair.
    ///
    /// Must be strictly less than `initial_margin_ratio`.
    ///
    /// When a user's equity falls below the sum of maintenance margins
    /// across all their positions, the user becomes eligible for liquidation.
    pub maintenance_margin_ratio: Ratio<UsdValue>,

    /// Minimum notional value (in USD) for the opening portion of an order.
    /// Notional = |opening_size| * oracle_price.
    ///
    /// Only enforced on the opening portion — closing is always allowed.
    pub min_opening_notional: UsdValue,

    /// Minimum price increment for orders in this pair.
    /// All limit prices must be a multiple of tick_size.
    pub tick_size: UsdPrice,

    /// Base half-spread used by the vault's market-making policy, as a
    /// fraction of oracle price.
    ///
    /// The vault places bids at `oracle_price * (1 - vault_half_spread)` and
    /// asks at `oracle_price * (1 + vault_half_spread)`.
    ///
    /// E.g., 0.0005 = 5 bps → for BTC at $100,000, the vault quotes
    /// $99,950 bid / $100,050 ask.
    pub vault_half_spread: Ratio<UsdPrice>,

    /// Maximum size per side that the vault will quote for this pair.
    /// Caps the vault's exposure growth per requote cycle.
    pub vault_max_quote_size: HumanAmount,
}
```

#### State

Global state:

```rust
struct State {
    /// The insurance fund balance.
    ///
    /// All PnL settlement flows through this fund. Funded by trading fees,
    /// liquidation fees, and surplus from liquidations.
    ///
    /// NOT part of LP claims — the vault's equity is computed separately via
    /// `compute_user_equity`.
    pub insurance_fund: BaseAmount,

    /// The vault's margin (LP-deposited capital).
    ///
    /// This is the vault's trading margin, used identically to any user's margin.
    /// The vault's equity is computed via `compute_user_equity(vault_state)`,
    /// which factors in unrealized PnL and accrued funding on vault positions.
    pub vault_margin: BaseAmount,

    /// Total supply of the vault's share token.
    pub vault_share_supply: BaseAmount,
}
```

Pair-specific state:

```rust
struct PairState {
    /// The sum of the sizes of all long positions.
    ///
    /// Should always be non-negative.
    pub long_oi: HumanAmount,

    /// The sum of the sizes of all short positions.
    ///
    /// Stored as a non-negative absolute value.
    pub short_oi: HumanAmount,

    /// Current instantaneous funding rate (fraction per day).
    ///
    /// Positive = longs pay shorts.
    /// Negative = shorts pay longs.
    pub funding_rate: Ratio<UsdValue, Duration>,

    /// Timestamp of last funding accrual.
    pub last_funding_time: Timestamp,

    /// Cumulative funding per unit of position size, denominated in USD.
    ///
    /// This is an ever-increasing accumulator. To compute a position's accrued
    /// funding, take the difference between the current value and the position's
    /// `entry_funding_per_unit`.
    pub cumulative_funding_per_unit: Ratio<UsdValue, HumanAmount>,

}
```

User-specific state:

```rust
struct UserState {
    // -------------------------------------------------------------------------
    // Counterparty Vault State
    // -------------------------------------------------------------------------

    /// The amount of vault shares this user owns.
    pub vault_shares: BaseAmount,

    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: Vec<Unlock>,

    // -------------------------------------------------------------------------
    // Trading State
    // -------------------------------------------------------------------------

    /// Trading collateral balance in settlement currency (e.g., USDT).
    pub margin: BaseAmount,

    /// Margin reserved for pending limit orders.
    pub reserved_margin: BaseAmount,

    /// Number of resting limit orders this user currently has on the book.
    pub open_order_count: u32,

    /// The user's open positions.
    pub positions: Map<PairId, Position>,
}

struct Position {
    /// Position size in human units.
    ///
    /// Positive = long position (profits when price increases).
    /// Negative = short position (profits when price decreases).
    pub size: HumanAmount,

    /// The average price at which this position was entered, in USD per
    /// human unit.
    ///
    /// Used for PnL calculation: PnL = size * (oracle_price - entry_price)
    pub entry_price: UsdPrice,

    /// Value of `pair_state.cumulative_funding_per_unit` when this position
    /// was last opened, modified, or had funding settled.
    pub entry_funding_per_unit: Ratio<UsdValue, HumanAmount>,
}

struct Unlock {
    /// The amount of settlement currency to be released to the user once
    /// cooldown completes.
    pub amount_to_release: BaseAmount,

    /// The time when cooldown completes.
    pub end_time: Timestamp,
}
```

The vault's trading state is stored as a `UserState` at a sentinel address:

```rust
/// The vault's user state is stored at the contract's own address.
/// Its `margin` field is aliased to `State.vault_margin`.
const VAULT_ADDR: UserId = /* contract's own address */;
```

#### Order

```rust
/// The Order struct stored as values in the indexed maps.
/// The key already encodes pair_id, limit_price, and created_at,
/// so the value only needs user_id, size, reduce_only, and reserved_margin.
struct Order {
    pub user_id: UserId,
    pub size: HumanAmount,
    pub reduce_only: bool,
    pub reserved_margin: BaseAmount,
}

struct OrderIndexes {
    /// Index the orders by order ID, so that an order can be retrieved by:
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

/// The set of all active pair IDs.
const PAIR_IDS: Item<Vec<PairId>> = Item::new("pair_ids");

/// Global state.
const STATE: Item<State> = Item::new("state");

/// Pair-specific states.
const PAIR_STATES: Map<PairId, PairState> = Map::new("pair_state");

/// User states.
const USER_STATES: Map<UserId, UserState> = Map::new("user_state");

/// Buy orders indexed for descending price iteration (most competitive first).
/// Key: (pair_id, inverted_limit_price, created_at, order_id)
/// where inverted_limit_price = UsdPrice::MAX - limit_price, so ascending iteration
/// yields descending prices.
const BIDS: IndexedMap<(PairId, UsdPrice, Timestamp, OrderId), Order> = IndexedMap::new("bid", OrderIndexes {
    order_id: UniqueIndex::new(/* ... */),
    user_id: MultiIndex::new(/* ... */),
});

/// Sell orders indexed for ascending price iteration (most competitive first).
/// Key: (pair_id, limit_price, created_at, order_id)
const ASKS: IndexedMap<(PairId, UsdPrice, Timestamp, OrderId), Order> = IndexedMap::new("ask", OrderIndexes {
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
    amount_received: BaseAmount,
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
    oracle_prices: &Map<PairId, UsdPrice>,
    amount: BaseAmount,
    current_time: Timestamp,
    usdt_price: Ratio<UsdValue, BaseAmount>,
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
      amount <= compute_available_margin(user_state, oracle_prices, pair_params_map, pair_states, usdt_price),
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
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let equity = compute_user_equity(user_state, oracle_prices, pair_states, usdt_price);
    let used = compute_used_margin(user_state, oracle_prices, pair_params_map, usdt_price);

    // equity - used - reserved, floored at zero.
    // Equity can be negative since unrealized PnL can make it so.
    let available = equity - used - user_state.reserved_margin;

    max(floor(available), 0)
}

/// Compute the margin currently used by open positions.
///
/// For each position, the used margin is:
///   |position_size| * current_price * initial_margin_ratio
fn compute_used_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let mut total: BaseAmount = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price: UsdPrice = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        // Used margin = |size| * price * initial_margin_ratio
        let margin: UsdValue = abs(position.size) * oracle_price * pair_params.initial_margin_ratio;

        total += ceil(margin / usdt_price); // Round up (conservative, disadvantage to user)
    }

    total
}

/// Compute total used margin with one position projected to a new size.
///
/// Identical to `compute_used_margin`, but overrides the size for
/// `projected_pair_id` with `projected_size`. If the user has no existing
/// position in that pair, adds it as a new entry.
///
/// Used by the post-fill margin check to validate that the user can cover
/// the projected position before executing.
fn compute_initial_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    projected_pair_id: PairId,
    projected_size: HumanAmount,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let mut total: BaseAmount = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price: UsdPrice = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        let size: HumanAmount = if pair_id == projected_pair_id {
            projected_size
        } else {
            position.size
        };

        let margin: UsdValue = abs(size) * oracle_price * pair_params.initial_margin_ratio;

        total += ceil(margin / usdt_price);
    }

    // If the projected pair is not among existing positions, add it.
    if !user_state.positions.contains_key(&projected_pair_id) && projected_size != HumanAmount::ZERO {
        let oracle_price: UsdPrice = oracle_prices[&projected_pair_id];
        let pair_params = pair_params_map[&projected_pair_id];
        let margin: UsdValue = abs(projected_size) * oracle_price * pair_params.initial_margin_ratio;

        total += ceil(margin / usdt_price);
    }

    total
}

/// Compute the unrealized PnL for a position at the current oracle price.
///
/// unrealized_pnl = size * (oracle_price - entry_price)
///
/// Long  (size > 0): profits when oracle_price > entry_price. ✓
/// Short (size < 0): profits when oracle_price < entry_price. ✓
fn compute_position_unrealized_pnl(
    position: &Position,
    oracle_price: UsdPrice,
) -> UsdValue {
    position.size * (oracle_price - position.entry_price)
}

/// Compute the user's equity: margin balance adjusted for unrealized PnL
/// and accrued funding across all open positions.
///
/// equity = margin + Σ unrealized_pnl - Σ accrued_funding
///
/// NOTE: For maximum accuracy, `accrue_funding` should be called for each
/// pair before invoking this function, so that `cumulative_funding_per_unit`
/// is up-to-date.
///
/// This function is used for BOTH regular users AND the vault. The vault's
/// equity is `compute_user_equity(vault_state)` — no special formula needed.
fn compute_user_equity(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let mut total_unrealized_pnl: UsdValue = UsdValue::ZERO;
    let mut total_accrued_funding: UsdValue = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price: UsdPrice = oracle_prices[&pair_id];
        let pair_state = &pair_states[&pair_id];

        total_unrealized_pnl += compute_position_unrealized_pnl(position, oracle_price);
        total_accrued_funding += compute_accrued_funding(position, pair_state);
    }

    // margin is BaseAmount; convert USD amounts via usdt_price for signed arithmetic
    user_state.margin + (total_unrealized_pnl - total_accrued_funding) / usdt_price
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
fn compute_required_margin(
    opening_size: HumanAmount,
    limit_price: UsdPrice,
    pair_params: &PairParams,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    if opening_size == HumanAmount::ZERO {
        return BaseAmount::ZERO;
    }

    let required_margin: UsdValue = abs(opening_size) * limit_price * pair_params.initial_margin_ratio;

    ceil(required_margin / usdt_price)
}
```

### Submit order

#### Matching engine

The core of the order book model. An incoming (taker) order walks the opposite side of the book, filling at each resting (maker) order's price. This gives the taker price improvement when resting orders are better than the limit price.

**Matching rules**:

- **Price-time priority**: best price first, then earliest timestamp at the same price level.
- **Fill at maker's price**: the taker always fills at the resting order's limit price.
- **Partial fills**: each resting order can be partially filled; the remainder stays on the book.

#### Decompose order into closing vs opening

On receiving an order, we first decompose it into closing and opening portions.

```rust
/// Returns (closing_size, opening_size) where closing_size reduces existing
/// exposure and opening_size creates new exposure.
/// Both have the same sign as the original size (or are zero).
fn decompose_fill(size: HumanAmount, user_pos: HumanAmount) -> (HumanAmount, HumanAmount) {
    if size == 0 {
        return (0, 0);
    }

    // Closing occurs when size and position have opposite signs
    if size > 0 && user_pos < 0 {
        // Buy order, user has short position
        let closing = min(size, -user_pos);
        let opening = size - closing;
        (closing, opening)
    } else if size < 0 && user_pos > 0 {
        // Sell order, user has long position
        let closing = max(size, -user_pos);
        let opening = size - closing;
        (closing, opening)
    } else {
        // No closing: size and position have same sign (or position is zero)
        (0, size)
    }
}
```

#### OI constraint check

```rust
/// Check that the opening portion of an order does not violate the max OI
/// constraint. If violated, the order is rejected entirely.
fn check_oi_constraint(
    opening_size: HumanAmount,
    pair_state: &PairState,
    max_abs_oi: HumanAmount,
) -> Result<()> {
    if opening_size > 0 {
        ensure!(pair_state.long_oi + opening_size <= max_abs_oi, "max long OI exceeded");
    } else if opening_size < 0 {
        ensure!(pair_state.short_oi + (-opening_size) <= max_abs_oi, "max short OI exceeded");
    }
    Ok(())
}
```

#### Validate notional constraints

```rust
/// Validate minimum opening notional constraint.
fn validate_notional_constraints(
    opening_size: HumanAmount,
    oracle_price: UsdPrice,
    pair_params: &PairParams,
) {
    if opening_size != HumanAmount::ZERO {
        let opening_notional: UsdValue = abs(opening_size) * oracle_price;
        ensure!(
            opening_notional >= pair_params.min_opening_notional,
            "opening notional below minimum"
        );
    }
}
```

#### Trading fees

A trading fee is charged on every voluntary fill as a percentage of notional value. The fee is transferred from the trader's margin to the insurance fund. Different rates apply to makers (resting orders) and takers (incoming orders).

- **Liquidation fills are exempt**: the existing `liquidation_fee_rate` is the only fee on liquidation.

```rust
/// Compute the trading fee for a given size and price.
fn compute_trading_fee(
    size: HumanAmount,
    price: UsdPrice,
    fee_rate: Ratio<UsdValue>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    ceil(abs(size) * price * fee_rate / usdt_price)
}

fn collect_trading_fee(
    state: &mut State,
    user_state: &mut UserState,
    fill_size: HumanAmount,
    fill_price: UsdPrice,
    fee_rate: Ratio<UsdValue>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let fee = compute_trading_fee(fill_size, fill_price, fee_rate, usdt_price);
    let actual_fee = min(fee, user_state.margin);

    user_state.margin -= actual_fee;
    state.insurance_fund += actual_fee;

    actual_fee
}
```

#### Match order (core matching engine)

The matching engine processes an incoming taker order against the opposite side of the book. It returns the total size filled and any unfilled remainder.

```rust
/// Process a single fill between taker and a resting maker order.
///
/// Handles PnL settlement, funding, OI updates, and fee collection for
/// both sides. The fill executes at the maker's limit price.
///
/// Returns the filled size (same sign as taker's order).
fn process_fill(
    params: &Params,
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    taker_state: &mut UserState,
    maker_state: &mut UserState,
    pair_id: PairId,
    fill_size: HumanAmount,     // size from taker's perspective
    fill_price: UsdPrice,       // maker's limit price
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    // ---- Taker side ----
    let taker_pos = taker_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(HumanAmount::ZERO);
    let (taker_closing, taker_opening) = decompose_fill(fill_size, taker_pos);

    execute_fill(state, pair_state, taker_state, pair_id, fill_price, taker_closing, taker_opening, usdt_price);
    collect_trading_fee(state, taker_state, fill_size, fill_price, params.taker_fee_rate, usdt_price);

    // ---- Maker side ----
    // The maker's fill is the opposite sign of the taker's.
    let maker_fill_size = -fill_size;
    let maker_pos = maker_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(HumanAmount::ZERO);
    let (maker_closing, maker_opening) = decompose_fill(maker_fill_size, maker_pos);

    execute_fill(state, pair_state, maker_state, pair_id, fill_price, maker_closing, maker_opening, usdt_price);
    collect_trading_fee(state, maker_state, fill_size, fill_price, params.maker_fee_rate, usdt_price);
}

/// Walk the opposite side of the book and match the incoming taker order.
///
/// Returns (total_filled_size, unfilled_size).
///
/// For a buy (taker) order, we walk ASKS (ascending price).
/// For a sell (taker) order, we walk BIDS (descending price).
///
/// Matching stops when:
/// - The taker order is fully filled.
/// - No more resting orders at an acceptable price.
/// - The price constraint is violated (limit price for GTC, slippage for market).
fn match_order(
    params: &Params,
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    taker_state: &mut UserState,
    pair_id: PairId,
    mut remaining_size: HumanAmount,  // positive = buy, negative = sell
    target_price: UsdPrice,           // worst acceptable price
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> (HumanAmount, HumanAmount) {
    let is_buy = remaining_size > 0;
    let mut total_filled: HumanAmount = HumanAmount::ZERO;

    // Select the opposite side of the book.
    let book_iter = if is_buy {
        ASKS.prefix(pair_id).range(..)  // ascending price (cheapest first)
    } else {
        BIDS.prefix(pair_id).range(..)  // descending price (most expensive first)
    };

    for (key, resting_order) in book_iter {
        // Recover the resting order's limit price.
        let resting_price: UsdPrice = if is_buy {
            key.limit_price
        } else {
            UsdPrice::MAX - key.inverted_limit_price
        };

        // Price check: stop if resting price is worse than target.
        // Buy: resting ask price > target → too expensive.
        // Sell: resting bid price < target → too cheap.
        if is_buy && resting_price > target_price {
            break;
        }
        if !is_buy && resting_price < target_price {
            break;
        }

        // Determine fill size: min of taker's remaining and maker's resting size.
        // Resting order size has the opposite sign of the taker.
        let resting_abs = abs(resting_order.size);
        let remaining_abs = abs(remaining_size);
        let fill_abs = min(remaining_abs, resting_abs);

        // Fill size from taker's perspective (preserving taker's sign).
        let fill_size = if is_buy { fill_abs } else { -fill_abs };

        // Decompose taker's fill to check OI constraint for opening portion.
        let taker_pos = taker_state.positions
            .get(&pair_id)
            .map(|p| p.size)
            .unwrap_or(HumanAmount::ZERO);
        let (_, taker_opening) = decompose_fill(fill_size, taker_pos);

        if check_oi_constraint(taker_opening, pair_state, pair_params.max_abs_oi).is_err() {
            // OI would be violated by the taker's opening portion.
            // Stop matching — further fills would also violate.
            break;
        }

        // Load maker state and execute the fill.
        let mut maker_state = load_user_state(resting_order.user_id);

        process_fill(
            params, state, pair_state, pair_params,
            taker_state, &mut maker_state,
            pair_id, fill_size, resting_price,
            oracle_prices, pair_params_map, pair_states, usdt_price,
        );

        // Update resting order: partial or full fill.
        if fill_abs >= resting_abs {
            // Resting order fully filled: remove from book.
            let order_book = if is_buy { &ASKS } else { &BIDS };
            order_book.remove(key);
            maker_state.reserved_margin -= resting_order.reserved_margin;
            maker_state.open_order_count -= 1;
        } else {
            // Resting order partially filled: reduce size in place.
            let order_book = if is_buy { &ASKS } else { &BIDS };
            let new_size = resting_order.size + fill_size;  // reduce by fill
            order_book.update(key, Order { size: new_size, ..resting_order });

            // Release proportional reserved margin.
            let released = resting_order.reserved_margin * fill_abs / resting_abs;
            maker_state.reserved_margin -= released;
        }

        save_user_state(maker_state);

        total_filled += fill_size;
        remaining_size -= fill_size;

        if remaining_size == HumanAmount::ZERO {
            break;
        }
    }

    (total_filled, remaining_size)
}
```

#### Store limit order (resting)

```rust
/// Store the unfilled portion of a limit order for later matching (GTC).
///
/// Validates the user's open order count, reserves margin and fee for the
/// full order size (worst-case), and persists the order in the appropriate
/// side of the order book.
fn store_limit_order(
    params: &Params,
    user_state: &mut UserState,
    pair_params: &PairParams,
    pair_id: PairId,
    user_id: UserId,
    unfilled_size: HumanAmount,
    limit_price: UsdPrice,
    reduce_only: bool,
    current_time: Timestamp,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    // Enforce maximum open orders
    ensure!(
        user_state.open_order_count < params.max_open_orders,
        "too many open orders"
    );

    // Enforce tick size.
    ensure!(
        limit_price % pair_params.tick_size == 0,
        "limit price must be a multiple of tick_size"
    );

    // Reserve margin and fee for the full order size as a worst-case estimate.
    let margin_to_reserve = compute_required_margin(unfilled_size, limit_price, pair_params, usdt_price);
    let fee_to_reserve = compute_trading_fee(unfilled_size, limit_price, params.taker_fee_rate, usdt_price);
    let reserved = margin_to_reserve + fee_to_reserve;

    // Check that the user has sufficient available margin to cover the reservation.
    let available_margin = compute_available_margin(user_state, oracle_prices, pair_params_map, pair_states, usdt_price);
    ensure!(available_margin >= reserved, "insufficient margin for limit order");

    user_state.reserved_margin += reserved;
    user_state.open_order_count += 1;

    // Store limit order as GTC
    let order = Order {
        user_id,
        size: unfilled_size,
        reduce_only,
        reserved_margin: reserved,
    };
    let order_id = generate_order_id();

    if unfilled_size > HumanAmount::ZERO {
        // Buy order: store with inverted price for descending iteration
        let key = (pair_id, UsdPrice::MAX - limit_price, current_time, order_id);
        BIDS.save(key, order);
    } else {
        // Sell order: store with normal price for ascending iteration
        let key = (pair_id, limit_price, current_time, order_id);
        ASKS.save(key, order);
    }
}
```

#### Putting things together

In V2, orders go through the matching engine. An incoming order walks the opposite side of the book, filling at each resting order's price. Any unfilled remainder either cancels (market/IOC) or rests in the book (limit/GTC).

1. Accrue funding.
2. Decompose the order into **closing** and **opening** portions. If `reduce_only`, zero out the opening portion.
3. Validate minimum opening notional.
4. Compute the target (worst acceptable) price.
5. Run the matching engine against the opposite side of the book.
6. If there is an unfilled remainder:
   - Market order: cancel the remainder (IOC behavior).
   - Limit order: place the remainder on the book (GTC behavior), with margin check.

```rust
fn handle_submit_order(
    params: &Params,
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    user_state: &mut UserState,
    user_id: UserId,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    pair_id: PairId,
    oracle_price: UsdPrice,
    size: HumanAmount,
    kind: OrderKind,
    reduce_only: bool,
    current_time: Timestamp,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    ensure!(size != 0, "nothing to do");

    // Step 1: Accrue funding before any OI changes
    accrue_funding(pair_state, pair_params, oracle_price, current_time);

    let user_pos: HumanAmount = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(HumanAmount::ZERO);
    let is_buy = size > 0;

    // Step 2: Decompose into closing and opening portions.
    let (closing_size, opening_size) = decompose_fill(size, user_pos);
    let opening_size = if reduce_only { HumanAmount::ZERO } else { opening_size };
    let order_size = closing_size + opening_size;
    ensure!(order_size != HumanAmount::ZERO, "order would have no effect");

    // Step 3: Validate minimum opening notional
    validate_notional_constraints(opening_size, oracle_price, pair_params);

    // Step 4: Compute target price (worst acceptable execution price).
    let target_price = match &kind {
        OrderKind::Market { max_slippage } => {
            if is_buy {
                oracle_price * (1 + *max_slippage)
            } else {
                oracle_price * (1 - *max_slippage)
            }
        }
        OrderKind::Limit { limit_price } => *limit_price,
    };

    // Step 5: Run matching engine.
    // The matching engine walks the opposite side of the book, filling at
    // each resting order's price. It handles OI checks per fill, executes
    // fills (PnL settlement, funding, position updates), and collects fees
    // for both sides.
    let (total_filled, unfilled) = match_order(
        params, state, pair_state, pair_params,
        user_state,
        pair_id, order_size, target_price,
        oracle_prices, pair_params_map, pair_states, usdt_price,
    );

    // Step 6: Handle unfilled remainder.
    if unfilled != HumanAmount::ZERO {
        match kind {
            OrderKind::Market { .. } => {
                // IOC: unfilled portion is canceled. No error — partial fills are ok.
                // If nothing filled at all and we wanted at least a fill, error.
                if total_filled == HumanAmount::ZERO {
                    bail!("no liquidity at acceptable price");
                }
            },
            OrderKind::Limit { limit_price } => {
                // GTC: place remainder on book. store_limit_order does its own margin check.
                store_limit_order(
                    params, user_state, pair_params, pair_id, user_id,
                    unfilled, limit_price, reduce_only,
                    current_time,
                    oracle_prices, pair_params_map, pair_states, usdt_price,
                );
            },
        }
    }
}
```

#### Execute fill

```rust
/// Execute a fill, updating positions and settling PnL and funding.
///
/// IMPORTANT: The caller must call `accrue_funding` before calling this
/// function. This ensures `cumulative_funding_per_unit` is up-to-date
/// before we settle per-position funding.
fn execute_fill(
    state: &mut State,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    pair_id: PairId,
    exec_price: UsdPrice,
    closing_size: HumanAmount,
    opening_size: HumanAmount,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    // ---- Settle funding if position exists ----
    if let Some(pos) = user_state.positions.get_mut(&pair_id) {
        settle_funding(state, pair_state, user_state, pos, usdt_price);
    }

    // ---- Closing-specific ----
    if closing_size != HumanAmount::ZERO {
        apply_closing(state, user_state, &pair_id, closing_size, exec_price, usdt_price);
    }

    // ---- Opening-specific ----
    if opening_size != HumanAmount::ZERO {
        apply_opening(user_state, pair_state, &pair_id, opening_size, exec_price);
    }

    // ---- Update OI ----
    update_oi(pair_state, opening_size, closing_size);
}

/// Close a portion of an existing position: realize PnL and reduce size.
fn apply_closing(
    state: &mut State,
    user_state: &mut UserState,
    pair_id: &PairId,
    closing_size: HumanAmount,
    exec_price: UsdPrice,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    let pos = user_state.positions.get_mut(pair_id).unwrap();

    // Realize PnL for the closing portion.
    let pnl = compute_pnl_to_realize(pos, closing_size, exec_price);
    settle_pnl(state, user_state, pnl, usdt_price);

    // Reduce position size.
    pos.size += closing_size;

    // Remove position if fully closed.
    if pos.size == HumanAmount::ZERO {
        user_state.positions.remove(pair_id);
    }
}

/// Grow an existing position or create a new one.
fn apply_opening(
    user_state: &mut UserState,
    pair_state: &PairState,
    pair_id: &PairId,
    opening_size: HumanAmount,
    exec_price: UsdPrice,
) {
    if let Some(pos) = user_state.positions.get_mut(pair_id) {
        let old_size = pos.size;
        pos.size += opening_size;

        if old_size == HumanAmount::ZERO {
            pos.entry_price = exec_price;
            pos.entry_funding_per_unit = pair_state.cumulative_funding_per_unit;
        } else {
            // Weighted average entry price.
            pos.entry_price = (abs(old_size) * pos.entry_price
                + abs(opening_size) * exec_price)
                / abs(pos.size);
        }
    } else {
        user_state.positions.insert(pair_id, Position {
            size: opening_size,
            entry_price: exec_price,
            entry_funding_per_unit: pair_state.cumulative_funding_per_unit,
        });
    }
}

/// Update `long_oi` and `short_oi` to reflect a fill.
fn update_oi(
    pair_state: &mut PairState,
    opening_size: HumanAmount,
    closing_size: HumanAmount,
) {
    // Opening portion increases OI on the respective side
    if opening_size > HumanAmount::ZERO {
        pair_state.long_oi += opening_size;
    } else if opening_size < HumanAmount::ZERO {
        pair_state.short_oi += opening_size.abs();
    }

    // Closing portion decreases OI on the opposite side
    if closing_size > HumanAmount::ZERO {
        pair_state.short_oi -= closing_size;
    } else if closing_size < HumanAmount::ZERO {
        pair_state.long_oi += closing_size;
    }
}

/// Compute the PnL to be realized when closing a portion of a position.
fn compute_pnl_to_realize(
    position: &Position,
    closing_size: HumanAmount,
    exec_price: UsdPrice,
) -> UsdValue {
    if closing_size == HumanAmount::ZERO || position.size == HumanAmount::ZERO {
        return UsdValue::ZERO;
    }

    let entry_value: UsdValue = abs(closing_size) * position.entry_price;
    let exit_value: UsdValue = abs(closing_size) * exec_price;

    if position.size > HumanAmount::ZERO {
        // Long position: profit when exit > entry
        exit_value - entry_value
    } else {
        // Short position: profit when entry > exit
        entry_value - exit_value
    }
}
```

#### PnL settlement (through insurance fund)

All PnL settlement flows through the insurance fund, for both regular users and the vault:

```rust
/// Settle realized PnL between user and insurance fund.
///
/// - Positive PnL (user wins): insurance fund pays user
/// - Negative PnL (user loses): user pays insurance fund
///
/// This function is used for ALL users, including the vault (whose margin
/// is `State.vault_margin`).
fn settle_pnl(
    state: &mut State,
    user_state: &mut UserState,
    pnl: UsdValue,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    if pnl > UsdValue::ZERO {
        // User wins: transfer from insurance fund to user
        let amount: BaseAmount = floor(pnl / usdt_price);
        state.insurance_fund = state.insurance_fund.saturating_sub(amount);
        user_state.margin += amount;
    } else if pnl < UsdValue::ZERO {
        // User loses: transfer from user to insurance fund
        let loss: BaseAmount = ceil(-pnl / usdt_price);
        let user_pays = min(loss, user_state.margin);
        user_state.margin -= user_pays;
        state.insurance_fund += user_pays;
    }
    // pnl == 0: no transfer needed
}
```

### Funding fee

Funding fees incentivize the market toward neutrality (equal long and short OI) by charging the majority side and paying the minority side. We use a **velocity-based** model (Synthetix V2/V3 style): the funding _rate_ changes over time at a velocity proportional to the current skew.

**Flow**: Bilateral. The majority side pays, the minority side receives. Funding settlement flows through the insurance fund (via `settle_pnl`).

#### Funding velocity

```rust
/// Compute the current funding velocity (rate of change of the funding rate).
///
/// velocity = (skew / skew_scale) * max_funding_velocity
fn compute_funding_velocity(
    pair_state: &PairState,
    pair_params: &PairParams,
) -> Ratio<Ratio<UsdValue, Duration>, Duration> {
    let skew: HumanAmount = pair_state.long_oi - pair_state.short_oi;
    (skew / pair_params.skew_scale) * pair_params.max_funding_velocity
}
```

#### Current funding rate

```rust
/// Compute the current funding rate, accounting for time elapsed since
/// the last accrual.
///
/// current_rate = clamp(last_rate + velocity * elapsed_days,
///                      -max_abs_funding_rate, max_abs_funding_rate)
fn compute_current_funding_rate(
    pair_state: &PairState,
    pair_params: &PairParams,
    current_time: Timestamp,
) -> Ratio<UsdValue, Duration> {
    let elapsed_secs = current_time - pair_state.last_funding_time;
    let elapsed_days = elapsed_secs / 86400;

    let velocity = compute_funding_velocity(pair_state, pair_params);
    let unclamped = pair_state.funding_rate + velocity * elapsed_days;

    clamp(unclamped, -pair_params.max_abs_funding_rate, pair_params.max_abs_funding_rate)
}
```

#### Unrecorded funding per unit

```rust
/// Compute the funding per unit of position size that has accrued since
/// the last accrual but not yet been recorded.
///
/// Returns `(unrecorded_funding_per_unit, current_rate)`.
///
/// Uses trapezoidal integration: the rate changes linearly from
/// `last_rate` to `current_rate` over the elapsed period.
fn compute_unrecorded_funding_per_unit(
    pair_state: &PairState,
    pair_params: &PairParams,
    oracle_price: UsdPrice,
    current_time: Timestamp,
) -> (Ratio<UsdValue, HumanAmount>, FundingRate) {
    let elapsed_secs = current_time - pair_state.last_funding_time;
    let elapsed_days = elapsed_secs / 86400;

    let current_rate = compute_current_funding_rate(pair_state, pair_params, current_time);
    let avg_rate = (pair_state.funding_rate + current_rate) / 2;

    (avg_rate * elapsed_days * oracle_price, current_rate)
}
```

#### Accruing funding (global)

```rust
/// Accrue funding for a pair: update the cumulative accumulator,
/// the current rate, and the timestamp.
///
/// MUST be called before any OI-changing operation to ensure correct accounting.
fn accrue_funding(
    pair_state: &mut PairState,
    pair_params: &PairParams,
    oracle_price: UsdPrice,
    current_time: Timestamp,
) {
    if current_time == pair_state.last_funding_time {
        return;
    }

    let (unrecorded, current_rate) = compute_unrecorded_funding_per_unit(
        pair_state,
        pair_params,
        oracle_price,
        current_time,
    );

    pair_state.funding_rate = current_rate;
    pair_state.last_funding_time = current_time;
    pair_state.cumulative_funding_per_unit += unrecorded;
}
```

#### Per-position accrued funding

```rust
/// Compute the funding accrued by a specific position since it was
/// last touched.
///
/// accrued = position.size * (current_cumulative - entry_cumulative)
///
/// Positive result = trader owes (cost). Negative = trader is owed (credit).
fn compute_accrued_funding(
    position: &Position,
    pair_state: &PairState,
) -> UsdValue {
    let delta: Ratio<UsdValue, HumanAmount> = pair_state.cumulative_funding_per_unit - position.entry_funding_per_unit;
    position.size * delta
}
```

#### Settling funding for a position

```rust
/// Settle accrued funding for a position. Transfers funds between the
/// trader's margin and the insurance fund, and resets the funding entry point.
fn settle_funding(
    state: &mut State,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    position: &mut Position,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    let accrued = compute_accrued_funding(position, pair_state);

    // Transfer funding between user and insurance fund.
    // Positive accrued = user pays. Negative = user receives.
    // We reuse settle_pnl with negated sign (accrued funding is a cost to the
    // trader, so it's negative PnL from their perspective).
    settle_pnl(state, user_state, -accrued, usdt_price);

    position.entry_funding_per_unit = pair_state.cumulative_funding_per_unit;
}
```

### Liquidation

When a user's equity drops below their total maintenance margin, any third party ("liquidator") may call `handle_liquidate`. Positions are closed via market orders through the matching engine. If the book cannot fully absorb the liquidation, the vault backstops the remaining position at oracle price.

#### Maintenance margin

```rust
/// Compute the total maintenance margin across all open positions.
fn compute_maintenance_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> BaseAmount {
    let mut total: BaseAmount = 0;

    for (pair_id, position) in &user_state.positions {
        let oracle_price: UsdPrice = oracle_prices[&pair_id];
        let pair_params = pair_params_map[&pair_id];

        let margin: UsdValue = abs(position.size) * oracle_price * pair_params.maintenance_margin_ratio;

        total += ceil(margin / usdt_price);
    }

    total
}
```

#### Liquidation check

```rust
/// Returns true if the user is eligible for liquidation.
fn is_liquidatable(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> bool {
    if user_state.positions.is_empty() {
        return false;
    }

    let equity = compute_user_equity(user_state, oracle_prices, pair_states, usdt_price);
    let maintenance_margin = compute_maintenance_margin(user_state, oracle_prices, pair_params_map, usdt_price);

    equity < maintenance_margin
}
```

#### Cancel all orders

```rust
/// Cancel all pending limit orders for a user across all pairs.
fn cancel_all_orders(user_state: &mut UserState, user_id: UserId) {
    for (key, _order) in BIDS.idx.user_id.prefix(user_id) {
        BIDS.remove(key);
    }

    for (key, _order) in ASKS.idx.user_id.prefix(user_id) {
        ASKS.remove(key);
    }

    user_state.reserved_margin = 0;
    user_state.open_order_count = 0;
}
```

#### Force close (liquidation handler)

```rust
/// Liquidate a user by closing all positions through the order book.
///
/// Can be called by any third party when the user is below maintenance margin.
///
/// Positions are closed as market orders through the matching engine.
/// If the book cannot fully absorb a position, the vault backstops the
/// remaining size at oracle price.
fn handle_liquidate(
    params: &Params,
    state: &mut State,
    pair_states: &mut Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    user_state: &mut UserState,
    user_id: UserId,
    current_time: Timestamp,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    // Step 1: Cancel all pending limit orders.
    cancel_all_orders(user_state, user_id);

    // Step 2: Accrue funding for all pairs the user has positions in.
    for (pair_id, _position) in &user_state.positions {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = pair_params_map[&pair_id];
        let oracle_price = oracle_prices[&pair_id];

        accrue_funding(pair_state, &pair_params, oracle_price, current_time);
    }

    // Step 3: Check liquidation condition.
    ensure!(
        is_liquidatable(user_state, oracle_prices, pair_params_map, pair_states, usdt_price),
        "user is not liquidatable",
    );

    // Step 4: Close all positions via market orders through the matching engine.
    //
    // For each position, submit the opposite size through the matching engine
    // with a permissive price (no slippage limit for liquidation).
    // If the book cannot fully absorb, the vault backstops at oracle price.
    let pair_ids: Vec<PairId> = user_state.positions.keys().cloned().collect();
    let mut total_notional: UsdValue = UsdValue::ZERO;

    for pair_id in pair_ids {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = pair_params_map[&pair_id];
        let oracle_price: UsdPrice = oracle_prices[&pair_id];
        let position_size: HumanAmount = user_state.positions[&pair_id].size;

        total_notional += abs(position_size) * oracle_price;

        // Close by filling the exact opposite of the current position.
        let close_size: HumanAmount = -position_size;
        let is_buy = close_size > 0;

        // Use extreme target price to ensure fills go through.
        let target_price = if is_buy { UsdPrice::MAX } else { UsdPrice::ZERO };

        let (filled, unfilled) = match_order(
            params, state, pair_state, &pair_params,
            user_state,
            pair_id, close_size, target_price,
            oracle_prices, pair_params_map, pair_states, usdt_price,
        );

        // Step 5: Vault backstop — if book couldn't fully absorb, the vault
        // takes the remaining position at oracle price.
        if unfilled != HumanAmount::ZERO {
            let mut vault_state = load_user_state(VAULT_ADDR);

            // Decompose for both liquidated user and vault.
            let user_pos = user_state.positions
                .get(&pair_id)
                .map(|p| p.size)
                .unwrap_or(HumanAmount::ZERO);
            let (user_closing, user_opening) = decompose_fill(unfilled, user_pos);
            execute_fill(state, pair_state, user_state, pair_id, oracle_price, user_closing, user_opening, usdt_price);

            let vault_fill = -unfilled;
            let vault_pos = vault_state.positions
                .get(&pair_id)
                .map(|p| p.size)
                .unwrap_or(HumanAmount::ZERO);
            let (vault_closing, vault_opening) = decompose_fill(vault_fill, vault_pos);
            execute_fill(state, pair_state, &mut vault_state, pair_id, oracle_price, vault_closing, vault_opening, usdt_price);

            save_user_state(vault_state);
        }

        // No trading fee on liquidation fills.
    }

    // Step 6: Pay liquidation fee to the insurance fund.
    //
    // The fee is proportional to total notional, capped at the user's
    // remaining margin.
    let fee = ceil(total_notional * params.liquidation_fee_rate / usdt_price);
    let actual_fee = min(fee, user_state.margin);

    user_state.margin -= actual_fee;
    state.insurance_fund += actual_fee;

    // Step 7: Bad debt check.
    //
    // If user_state.margin is still negative after all settlements (which would
    // manifest as margin already being 0 from settle_pnl's saturating_sub),
    // the bad debt has been absorbed by the insurance fund.
    // If the insurance fund is depleted, ADL may be triggered.
}
```

### Auto-deleveraging

Auto-deleveraging (ADL) triggers when the **insurance fund cannot cover bad debt** from a liquidation (Binance-style). Unlike spec.md's ratio-based trigger, ADL here is a last-resort mechanism activated only when the insurance fund is depleted.

The most profitable opposing position is selected (ranked by PnL% x effective leverage) and forcibly closed at the **bankruptcy price** (the price at which the liquidated user's margin = 0).

```rust
/// Compute the ADL ranking score for a position.
///
/// score = pnl_pct * effective_leverage
///       = (unrealized_pnl / margin) * (notional / margin)
///
/// Higher score = closed first during ADL.
/// Only positions with positive unrealized PnL are eligible.
fn compute_adl_score(
    position: &Position,
    oracle_price: UsdPrice,
    user_equity: BaseAmount,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> Ratio<UsdValue> {
    let unrealized_pnl = compute_position_unrealized_pnl(position, oracle_price);

    // Only profitable positions are eligible for ADL.
    if unrealized_pnl <= UsdValue::ZERO || user_equity == BaseAmount::ZERO {
        return Ratio::ZERO;
    }

    let notional: UsdValue = abs(position.size) * oracle_price;
    let pnl_pct = unrealized_pnl / (user_equity * usdt_price);
    let effective_leverage = notional / (user_equity * usdt_price);

    pnl_pct * effective_leverage
}

/// Compute the bankruptcy price for a position.
///
/// The bankruptcy price is the price at which the position's margin
/// (after funding) would be exactly zero. This is the price at which
/// the ADL'd counterparty's position is closed.
///
/// For longs:  bankruptcy_price = entry_price - margin_per_unit
/// For shorts: bankruptcy_price = entry_price + margin_per_unit
///
/// where margin_per_unit accounts for the user's total margin allocated
/// to this position (approximated proportionally by notional share).
fn compute_bankruptcy_price(
    position: &Position,
    user_margin: BaseAmount,
    total_notional: UsdValue,
    oracle_price: UsdPrice,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) -> UsdPrice {
    let pos_notional: UsdValue = abs(position.size) * oracle_price;
    // Allocate margin proportionally by notional share.
    let margin_for_pos: UsdValue = (user_margin * usdt_price) * pos_notional / total_notional;
    let margin_per_unit: UsdPrice = margin_for_pos / abs(position.size);

    if position.size > HumanAmount::ZERO {
        // Long: bankrupt when price drops by margin_per_unit
        position.entry_price - margin_per_unit
    } else {
        // Short: bankrupt when price rises by margin_per_unit
        position.entry_price + margin_per_unit
    }
}

/// Auto-deleverage a user's position when the insurance fund is depleted.
///
/// Triggered when a liquidation produces bad debt that exceeds the
/// insurance fund balance.
fn handle_deleverage(
    params: &Params,
    state: &mut State,
    pair_states: &mut Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
    user_state: &mut UserState,
    pair_id: PairId,
    current_time: Timestamp,
) {
    // Step 1: Accrue funding for this pair.
    let pair_state = pair_states.get_mut(&pair_id);
    let pair_params = pair_params_map[&pair_id];
    let oracle_price = oracle_prices[&pair_id];

    accrue_funding(pair_state, &pair_params, oracle_price, current_time);

    // Step 2: Verify insurance fund is depleted (bad debt exists).
    //
    // ADL only triggers when the insurance fund cannot cover bad debt.
    // This is checked by verifying the insurance fund is at zero after
    // a liquidation that produced bad debt.
    ensure!(
        state.insurance_fund == BaseAmount::ZERO,
        "insurance fund is not depleted, ADL not needed",
    );

    // Step 3: Verify the user has a position in this pair.
    ensure!(
        user_state.positions.contains_key(&pair_id),
        "user has no position in this pair",
    );

    // Step 4: Verify the user's position is profitable (ADL targets
    // the most profitable opposing positions).
    let position = &user_state.positions[&pair_id];
    let unrealized_pnl = compute_position_unrealized_pnl(position, oracle_price);
    ensure!(
        unrealized_pnl > UsdValue::ZERO,
        "position is not profitable, not eligible for ADL",
    );

    // Step 5: Compute bankruptcy price and close at that price.
    //
    // The bankruptcy price is the price at which the liquidated user's
    // margin = 0. Closing at this price means the ADL'd trader absorbs
    // the loss that the insurance fund couldn't cover.
    let total_notional = compute_total_user_notional(user_state, oracle_prices);
    let bankruptcy_price = compute_bankruptcy_price(
        position, user_state.margin, total_notional, oracle_price, usdt_price,
    );

    let position_size: HumanAmount = position.size;
    let fill_size: HumanAmount = -position_size;

    execute_fill(state, pair_state, user_state, pair_id, bankruptcy_price, fill_size, HumanAmount::ZERO, usdt_price);

    // No fee charged — the ADL'd trader is not at fault.
}

/// Compute total notional across all of a user's positions.
fn compute_total_user_notional(
    user_state: &UserState,
    oracle_prices: &Map<PairId, UsdPrice>,
) -> UsdValue {
    let mut total: UsdValue = UsdValue::ZERO;
    for (pair_id, position) in &user_state.positions {
        total += abs(position.size) * oracle_prices[&pair_id];
    }
    total
}
```

### Counterparty vault

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT to receive newly minted shares, or burn shares to redeem USDT. The vault is a **regular trader** on the order book — its equity is computed identically to any user via `compute_user_equity`.

#### Vault equity

Since the vault is a regular trader with explicit positions, its equity is computed using the same `compute_user_equity` function as any user:

```rust
/// Compute the vault's equity.
///
/// The vault is stored as a UserState at VAULT_ADDR. Its equity is computed
/// identically to any user: margin + unrealized_pnl - accrued_funding.
///
/// No special formula, no aggregate accumulators needed.
fn compute_vault_equity(
    pair_states: &Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
    current_time: Timestamp,
) -> BaseAmount {
    let vault_state = load_user_state(VAULT_ADDR);
    compute_user_equity(&vault_state, oracle_prices, pair_states, usdt_price)
}
```

#### Handling deposit

We use the following constant parameters to prevent the [ERC-4626 frontrunning donation attack](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v5.5.0/contracts/token/ERC20/extensions/ERC4626.sol#L22-L48):

```rust
/// Virtual shares added to total supply in share price calculations.
const VIRTUAL_SHARES: BaseAmount = 1_000_000;

/// Virtual assets added to vault equity in share price calculations.
const VIRTUAL_ASSETS: BaseAmount = 1;
```

```rust
fn handle_deposit_liquidity(
    state: &mut State,
    pair_states: &Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    user_state: &mut UserState,
    amount_received: BaseAmount,
    min_shares_to_mint: Option<BaseAmount>,
    usdt_price: Ratio<UsdValue, BaseAmount>,
    current_time: Timestamp,
) {
    ensure!(amount_received > 0, "nothing to do");

    let effective_supply = state.vault_share_supply + VIRTUAL_SHARES;
    let effective_equity = compute_vault_equity(
        pair_states, pair_params_map, oracle_prices, usdt_price, current_time,
    ) + VIRTUAL_ASSETS;

    ensure!(
        effective_equity > 0,
        "vault is in catastrophic loss! deposit disabled"
    );

    let shares_to_mint = floor(amount_received * effective_supply / effective_equity);

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

```rust
fn handle_unlock_liquidity(
    state: &mut State,
    pair_states: &Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    user_state: &mut UserState,
    shares_to_burn: BaseAmount,
    usdt_price: Ratio<UsdValue, BaseAmount>,
    current_time: Timestamp,
) {
    ensure!(shares_to_burn > 0, "nothing to do");
    ensure!(user_state.vault_shares >= shares_to_burn, "can't burn more than what you have");

    let vault_equity = compute_vault_equity(
        pair_states, pair_params_map, oracle_prices, usdt_price, current_time,
    );

    ensure!(
        vault_equity > 0,
        "vault is in catastrophic loss! withdrawal disabled"
    );

    let effective_supply = state.vault_share_supply + VIRTUAL_SHARES;
    let effective_equity = vault_equity + VIRTUAL_ASSETS;
    let amount_to_release = floor(effective_equity * shares_to_burn / effective_supply);

    ensure!(
        state.vault_margin >= amount_to_release,
        "the vault doesn't have sufficient balance to fulfill this withdrawal"
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

### Vault requoting policy

The vault follows a simple on-chain market-making policy triggered on each oracle update. It cancels all existing vault orders and places new bid/ask orders through the matching engine. When these vault orders enter the matching engine, they naturally match against resting user limit orders that have become "in the money" due to the oracle price change.

```rust
/// Execute the vault's requoting policy for a single pair.
///
/// Called during `on_oracle_update` for each active pair.
///
/// 1. Cancel all existing vault orders for this pair.
/// 2. Compute bid/ask prices from oracle + spread.
/// 3. Compute available size from vault equity and margin.
/// 4. Submit vault bid and ask through the matching engine.
fn vault_requote(
    params: &Params,
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    pair_id: PairId,
    oracle_price: UsdPrice,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_states: &Map<PairId, PairState>,
    num_active_pairs: u32,
    current_time: Timestamp,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    let mut vault_state = load_user_state(VAULT_ADDR);

    // Step 1: Cancel all existing vault orders for this pair.
    // (Only cancel for this pair, not all pairs — other pairs are requoted
    // in their own iteration.)
    for (key, order) in BIDS.idx.user_id.prefix(VAULT_ADDR) {
        if key.pair_id == pair_id {
            vault_state.reserved_margin -= order.reserved_margin;
            vault_state.open_order_count -= 1;
            BIDS.remove(key);
        }
    }
    for (key, order) in ASKS.idx.user_id.prefix(VAULT_ADDR) {
        if key.pair_id == pair_id {
            vault_state.reserved_margin -= order.reserved_margin;
            vault_state.open_order_count -= 1;
            ASKS.remove(key);
        }
    }

    // Step 2: Compute prices.
    let bid_price = oracle_price * (1 - pair_params.vault_half_spread);
    let ask_price = oracle_price * (1 + pair_params.vault_half_spread);

    // Round to tick size.
    let bid_price = floor_to_tick(bid_price, pair_params.tick_size);
    let ask_price = ceil_to_tick(ask_price, pair_params.tick_size);

    // Step 3: Compute available quote size.
    let vault_equity = compute_user_equity(&vault_state, oracle_prices, pair_states, usdt_price);
    let used_margin = compute_used_margin(&vault_state, oracle_prices, pair_params_map, usdt_price);
    let available_margin = max(0, vault_equity - used_margin - vault_state.reserved_margin);

    // Divide available margin evenly across active pairs.
    let margin_for_this_pair = available_margin / num_active_pairs;

    // Convert margin to position size.
    // margin = size * price * initial_margin_ratio
    // size = margin / (price * initial_margin_ratio)
    let available_size = floor(
        margin_for_this_pair / (oracle_price * pair_params.initial_margin_ratio) * usdt_price
    );

    let quote_size = min(pair_params.vault_max_quote_size, available_size);

    if quote_size <= HumanAmount::ZERO {
        save_user_state(vault_state);
        return;
    }

    // Step 4: Submit vault bid and ask through the matching engine.
    //
    // These go through the standard matching engine, so they naturally
    // match against resting user limit orders that are now "in the money"
    // at the new oracle price.

    // Submit buy order (positive size).
    let buy_size = quote_size;
    let (_, buy_unfilled) = match_order(
        params, state, pair_state, pair_params,
        &mut vault_state,
        pair_id, buy_size, bid_price,
        oracle_prices, pair_params_map, pair_states, usdt_price,
    );

    // Place any unfilled buy remainder on the book.
    if buy_unfilled > HumanAmount::ZERO {
        store_limit_order(
            params, &mut vault_state, pair_params, pair_id, VAULT_ADDR,
            buy_unfilled, bid_price, false,
            current_time,
            oracle_prices, pair_params_map, pair_states, usdt_price,
        );
    }

    // Submit sell order (negative size).
    let sell_size = -quote_size;
    let (_, sell_unfilled) = match_order(
        params, state, pair_state, pair_params,
        &mut vault_state,
        pair_id, sell_size, ask_price,
        oracle_prices, pair_params_map, pair_states, usdt_price,
    );

    // Place any unfilled sell remainder on the book.
    if sell_unfilled < HumanAmount::ZERO {
        store_limit_order(
            params, &mut vault_state, pair_params, pair_id, VAULT_ADDR,
            sell_unfilled, ask_price, false,
            current_time,
            oracle_prices, pair_params_map, pair_states, usdt_price,
        );
    }

    save_user_state(vault_state);
}

/// Round price down to the nearest tick.
fn floor_to_tick(price: UsdPrice, tick_size: UsdPrice) -> UsdPrice {
    floor(price / tick_size) * tick_size
}

/// Round price up to the nearest tick.
fn ceil_to_tick(price: UsdPrice, tick_size: UsdPrice) -> UsdPrice {
    ceil(price / tick_size) * tick_size
}
```

### On oracle update

At the beginning of each block, validators submit the latest oracle prices. The contract is triggered to accrue funding, release matured unlocks, and execute the vault's requoting policy.

```rust
fn on_oracle_update(
    params: &Params,
    state: &mut State,
    pair_states: &mut Map<PairId, PairState>,
    pair_params_map: &Map<PairId, PairParams>,
    oracle_prices: &Map<PairId, UsdPrice>,
    pair_ids: &[PairId],
    current_time: Timestamp,
    usdt_price: Ratio<UsdValue, BaseAmount>,
) {
    let num_active_pairs = pair_ids.len() as u32;

    for &pair_id in pair_ids {
        let pair_state = pair_states.get_mut(&pair_id);
        let pair_params = &pair_params_map[&pair_id];
        let oracle_price = oracle_prices[&pair_id];

        // Step 1: Accrue funding.
        accrue_funding(pair_state, pair_params, oracle_price, current_time);

        // Step 2: Run vault requoting policy.
        // The vault cancels old orders and places new bid/ask through the
        // matching engine. This naturally matches against resting user orders
        // that are now in the money.
        vault_requote(
            params, state, pair_state, pair_params,
            pair_id, oracle_price,
            oracle_prices, pair_params_map, pair_states,
            num_active_pairs, current_time, usdt_price,
        );
    }

    // Step 3: Release matured unlocks.
    // Iterate all users with pending unlocks and release any that have
    // passed their end_time. This is trivial and implementation-specific,
    // so we omit the details here.
    release_matured_unlocks(current_time);
}
```

## Out-of-scope features

The following are out-of-scope for now, but may be added in the future:

v2.5 (near future):

- **Isolated margin**
- **TP/SL**: automatically close the position if PnL reaches an upper or lower threshold.
- **Partial liquidation**: reduce positions incrementally until the user is back above maintenance margin, rather than closing all positions at once.

v3 (further future):

- **Batch auctions**: periodic matching instead of continuous, to reduce MEV.
- **Fee tiers**: volume-based maker/taker fee discounts.
- **Advanced vault strategies**: inventory-aware spread adjustment, skew-based widening, dynamic sizing based on market conditions.
