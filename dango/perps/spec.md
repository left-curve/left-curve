# Perpetual futures exchange: specifications

- This is a perpetual futures (perps) exchange that uses the **peer-to-pool model**, similar to e.g. Ostium. A liquidity pool provides quotes (based on data including oracle price, open interest (OI), and the order's size). All orders are executed against the pool, with the pool taking the counterparty position (e.g. if a user opens a long position of 5 BTC, the pool takes the opposite: a short position of 5 BTC). We call the pool the **counterparty pool**. Empirically, traders in aggregate lose money in the long run, which means counterparty pool makes money. This is in contrary to the peer-to-peer model, where users place orders in an order book; a user's order is executed against other users' orders.
- The exchange operates in **one-way mode**. Meaning, e.g., a user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, the position is modified. This is in contrary to **two-way mode**, where a user can have multiple positions in a single asset. When placing an order, the user can choose whether the order will create a new position or modify an existing one.
- To ensure the protocol's solvency and profitability, it's important the market is close to _neutral_, meaning there is roughly the same amount of long and short OI. We incentivize this through two mechanisms, **skew pricing** and **funding fee** (described in respective sections).
- For now, the exchange supports only **cross margin**. Support for isolated margin may be added in a future update. The exchange uses **margin reservation at order placement**: when submitting an order, the required margin is calculated upfront and reserved, preventing withdrawal. This eliminates the need to check margin at execution time and ensures predictable order outcomes.

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
    // Liquidity Provider (LP) Methods
    // -------------------------------------------------------------------------

    /// Add liquidity to the counterparty vault.
    ///
    /// User must send a non-zero amount of the settlement currency (defined in
    /// `Params` below) as attachment of this function call.
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
    // Trader Methods
    // -------------------------------------------------------------------------

    /// Deposit funds into the user's trading balance.
    ///
    /// User must send a non-zero amount of the settlement currency as attachment
    /// of this function call.
    ///
    /// This is for **trading**, not liquidity provision. The trading balance
    /// serves as collateral for opening and maintaining positions.
    DepositMargin {},

    /// Withdraw funds from the user's trading balance.
    ///
    /// Can only withdraw up to the available margin (total balance minus used
    /// margin minus reserved margin).
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
        /// contract represents 1 BTC, and the user specifies a `size` of 1, it
        /// means the user wishes to increase his long exposure or decrease his
        /// short exposure by 1 BTC.
        ///
        /// Positive for buy (increase long exposure, decrease short exposure);
        /// negative for sell (decrease long exposure, increase short exposure).
        size: Dec,

        /// The order type: market, limit, etc.
        kind: OrderKind,

        /// If true, only the closing portion of the order is executed; any
        /// opening portion is discarded. If false, if the opening portion
        /// would violate OI constraints, the entire order (including the
        /// closing portion) is reverted.
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
    /// Trade at the current price quoted by the counterparty pool, optionally
    /// with a slippage tolerance.
    ///
    /// If it's not possible to fill the order in full, the unfilled portion is
    /// canceled.
    Market {
        /// The execution price must not be worse than the _marginal price_ plus
        /// this slippage.
        ///
        /// Marginal price is the price that the counterparty pool may quote for
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

### Parameters

The global parameters apply to all trading pairs:

```rust
struct Params {
    /// Denomination of the asset used for the settlement of perpetual futures
    /// contracts. Typically a USD stablecoin.
    pub settlement_denom: Denom,

    /// The waiting period between a withdrawal from the counterparty vault is
    /// requested and is fulfilled.
    pub vault_cooldown_period: Duration,

    // TODO: max number of open order per user
}
```

Each trading pair is also associated with a set of pair-specific parameters:

```rust
struct PairParams {
    /// A scaling factor that determines how greatly an imbalance in open
    /// interest ("skew") should affect an order's execution price.
    /// The greater the value of the scaling factor, the less the effect.
    pub skew_scale: Udec,

    /// The maximum extend to which skew can affect execution price.
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

    /// Initial margin ratio for this pair.
    ///
    /// Determines the minimum collateral required to open a position.
    /// E.g., 0.05 = 5% = 20x maximum leverage.
    ///
    /// Required margin = position_size * price * initial_margin_ratio
    pub initial_margin_ratio: Udec,

    // TODO: min order size (either in number of contracts or in USD; to be decided)
}
```

### State

Global state:

```rust
struct State {
    /// The sum of all user deposits and the vault's realized PnL.
    ///
    /// Note that this doesn't equal the amount of funds withdrawable by burning
    /// shares, which also needs to factor in the vault's _unrealized_ PnL.
    pub vault_balance: Uint,

    /// Total supply of the vault's share token.
    pub vault_share_supply: Uint,

    // TODO: accumulator variables for computing the vault's PnL
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
}
```

User-specific state:

```rust
struct UserState {
    // -------------------------------------------------------------------------
    // Liquidity Provider (LP) fields
    // -------------------------------------------------------------------------

    /// The amount of vault shares this user owns.
    pub vault_shares: Uint,

    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: Vec<Unlock>,

    // -------------------------------------------------------------------------
    // Trader fields
    // -------------------------------------------------------------------------

    /// Trading collateral balance in settlement currency (e.g., USDT).
    pub margin: Uint,

    /// Margin reserved for pending limit orders.
    ///
    /// When a limit order is placed but not immediately filled, the required
    /// margin is reserved (locked) to ensure the user can cover the position
    /// if/when the order executes.
    pub reserved_margin: Uint,

    /// The user's open positions.
    pub positions: Map<PairId, Position>,
}

struct Position {
    /// Position size in contracts.
    ///
    /// Positive = long position (profits when price increases).
    /// Negative = short position (profits when price decreases).
    pub size: Dec,

    /// Total cost to open this position, in settlement currency.
    ///
    /// Used for PnL calculation: PnL = current_value - cost_basis
    pub cost_basis: Uint,

    // ... other fields TBD (e.g., entry price, realized PnL, funding accumulated)
}

struct Unlock {
    /// The amount of settlement currency to be released to the user once
    /// cooldown completes.
    pub amount_to_release: Uint,

    /// The time when cooldown completes.
    pub end_time: Timestamp,
}
```

GTC limit orders are stored in indexed maps for efficient scanning:

```rust
/// Buy orders indexed for descending price iteration (most competitive first).
/// Key: (pair_id, inverted_limit_price, created_at, order_id)
/// where inverted_limit_price = MAX_PRICE - limit_price, so ascending iteration
/// yields descending prices.
const BUY_ORDERS: Map<(PairId, Udec, Timestamp, OrderId), Order> = Map::new("bids");

/// Sell orders indexed for ascending price iteration (most competitive first).
/// Key: (pair_id, limit_price, created_at, order_id)
const SELL_ORDERS: Map<(PairId, Udec, Timestamp, OrderId), Order> = Map::new("asks");

/// The Order struct stored as values in the indexed maps.
/// The key already encodes pair_id, limit_price, and created_at,
/// so the value only needs user_id, size, and reduce_only.
struct Order {
    pub user_id: UserId,
    pub size: Dec,
    pub reduce_only: bool,
}
```

## Business logic

For simplicity, we assume the settlement asset (defined by `settlement_denom` in `Params`) is USDT.

### Vault deposit

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT coins to receive newly minted shares, or burn shares to redeem USDT coins.

At any time, the vault's **equity** is defined as the vault's token balance (which reflects the total amount of deposit from liquidity providers and the vault's realized PnL) plus its unrealized PnL:

```rust
fn compute_vault_equity(state: &State, usdt_price: Udec) -> Dec {
    state.vault_balance + (compute_vault_unrealized_pnl() / usdt_price)
}
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
    const DEFAULT_SHARES_PER_AMOUNT: Uint = /* can be any reasonable value */;

    ensure!(amount_received > 0, "nothing to do");

    // Compute the number of shares to mint.
    let shares_to_mint = if state.vault_share_supply != 0 {
        let vault_equity = compute_vault_equity(state, usdt_price);

        ensure!(
            vault_equity > 0,
            "vault is in catastrohpic loss! deposit disabled"
            // If the vault has positive shares (i.e. it isn't empty) but zero or
            // negative equity, the protocol is insolvent, and require intervention
            // e.g. a bailout. Disable deposits in this case.
        );

        // Round the number down, to the advantage of the protocol and disadvantage
        // of the user. This is a principle we must follow throughout the codebase.
        floor(amount_received * state.vault_share_supply / vault_equity)
    } else {
        floor(amount_received * DEFAULT_SHARES_PER_AMOUNT)
    };

    // Ensure the number of shares to mint is no less than the minimum.
    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "to few shares would be minted"
        );
    }

    // Update global state.
    state.vault_balance += amount_received;
    state.vault_share_supply += shares_to_mint;

    // Update user state.
    user_state.vault_shares += shares_to_mint;
}
```

### Vault withdrawal

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

    // Again, note the direction of rounding.
    let amount_to_release = floor(vault_equity * shares_to_burn / state.vault_share_supply);

    ensure!(
        state.vault_balance >= amount_to_release,
        "the vault doesn't have sufficient balance to fulfill with this withdrawal"
        // This can happen if the vault has a very positive unrealized PnL.
        // In this case, the liquidity provider must wait until that PnL is realized
        // (i.e. the losing positions from traders are either closed or liquidated)
        // before withdrawing.
    );

    // Update global state.
    state.vault_balance -= amount_to_release;
    state.vault_share_supply -= shares_to_burn;

    // Update user state.
    user_state.vault_shares -= shares_to_burn;

    // Insert the new unlock into the user's state.
    let end_time = current_time + params.vault_cooldown_period;
    user_state.unlocks.push(Unlock { amount_to_release, end_time });
}
```

Once the cooldown period elapses, the contract needs to be triggered to release the fund and remove this unlock. This is trivial and we ignore it in this spec.

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
    user_state: &mut UserState,
    amount: Uint,
) {
    ensure!(amount > 0, "nothing to do");

    ensure!(
      amount <= compute_available_margin(user_state),
      "insufficient available margin"
    );

    user_state.margin -= amount;

    // Transfer `amount` of settlement currency to user.
}
```

Margin is the collateral required to open and maintain positions. We distinguish three types:

- **Used margin**: Collateral currently backing open positions
- **Reserved margin**: Collateral locked for pending limit orders (not yet filled)
- **Available margin**: `balance - used_margin - reserved_margin`

```rust
/// Compute the margin available for withdrawal.
fn compute_available_margin(
    user_state: &UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
) -> Uint {
    user_state.margin
        .saturating_sub(compute_used_margin(user_state, oracle_prices, pair_params_map))
        .saturating_sub(user_state.reserved_margin)
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
```

### Cancel order

Users can cancel pending limit orders to release their reserved margin:

```rust
fn handle_cancel_order(
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

    // Compute and release the reserved margin for this order
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let (_, opening_size) = decompose_fill(order.size, user_pos);

    // For cancellation, we need to recover the limit_price from the order key
    // (implementation detail: the order key contains the limit price)
    let reserved = compute_required_margin(opening_size, limit_price, pair_params);
    user_state.reserved_margin = user_state.reserved_margin.saturating_sub(reserved);
}

/// Compute the margin required for the opening portion of an order.
///
/// Only the opening portion (new exposure) requires margin.
/// The closing portion releases margin.
fn compute_required_margin(
    opening_size: Dec,
    worst_case_price: Udec,
    pair_params: &PairParams,
) -> Uint {
    if opening_size == Dec::ZERO {
        return Uint::ZERO;
    }

    // Required margin = |opening_size| * worst_case_price * initial_margin_ratio
    let margin = abs(opening_size) * worst_case_price * pair_params.initial_margin_ratio;

    ceil(margin)  // Round up to be conservative (disadvantage to user)
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
- **`reduce_only = false`**: If the opening portion would violate OI constraints, the entire order (including any closing portion) is reverted. This is all-or-nothing semantics.

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
- If `skew` is negative, it means all traders combined have a net short exposure, and the counterparty vault has a net long exposure. To incentivize traders to go bakc to neutral, the vault will offer better prices for buying, and worse price for selling.
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

/// Maginal price is the execution price of an order of infinitesimal size.
/// This is requivalent to calling
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
fn clamp(x: Dec, min: Dec, max: Dex) -> Dec {
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
```

#### Putting things together

```rust
fn handle_submit_order(
    state: &mut State,
    pair_state: &mut PairState,
    pair_params: &PairParams,
    user_state: &mut UserState,
    oracle_prices: &Map<PairId, Udec>,
    pair_params_map: &Map<PairId, PairParams>,
    pair_id: PairId,
    oracle_price: Udec,
    size: Dec,
    kind: OrderKind,
    reduce_only: bool,
) {
    ensure!(size != 0, "nothing to do");

    let skew = pair_state.long_oi + pair_state.short_oi;
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let is_buy = size > 0;

    // Step 1: Decompose into closing and opening portions
    let (closing_size, opening_size) = decompose_fill(size, user_pos);

    // Step 2: Check OI constraint on opening portion
    let max_opening_from_oi = compute_max_opening_from_oi(opening_size, pair_state, pair_params.max_abs_oi);

    let oi_violated = if is_buy {
        max_opening_from_oi < opening_size
    } else {
        max_opening_from_oi > opening_size
    };

    // Step 3: Determine fill based on reduce_only flag and OI constraint
    let fill_size = if oi_violated {
        if reduce_only {
            // reduce_only=true: execute only closing portion, discard opening
            closing_size
        } else {
            // reduce_only=false: order is unfillable
            0
        }
    } else {
        // OI not violated: execute full order
        size
    };

    // Step 4: Compute the target price. This is the worst possible price the
    // order may be filled at.
    let target_price = compute_target_price(&kind, oracle_price, skew, pair_params, is_buy);

    // Step 5: Compute required margin for the opening portion of the ENTIRE order
    // (not just fill_size, but the full opening_size including unfilled)
    let (_, full_opening_size) = decompose_fill(size, user_pos);
    let required_margin = compute_required_margin(full_opening_size, target_price, pair_params);

    // Step 6: Check available margin BEFORE any execution
    let available_margin = compute_available_margin(user_state, oracle_prices, pair_params_map);
    ensure!(available_margin >= required_margin, "insufficient margin");

    // Step 7: Check price constraint (all-or-nothing on entire fill)
    if fill_size != Dec::ZERO {
        let exec_price = compute_exec_price(oracle_price, skew, fill_size, pair_params);

        let price_ok = if is_buy {
            exec_price <= target_price
        } else {
            exec_price >= target_price
        };

        if price_ok {
            // Step 8: Execute the fill
            execute_fill(state, pair_state, user_state, pair_id, fill_size, exec_price);
        }
    }

    // Step 9: Handle unfilled portion based on order kind
    let unfilled_size = size - fill_size;
    if unfilled_size != Dec::ZERO {
        match kind {
            OrderKind::Market { .. } => {
                // Market orders are IOC: discard unfilled portion (no-op)
            },
            OrderKind::Limit { limit_price } => {
                // Step 10: Reserve margin for the unfilled portion
                // The filled portion's margin is now "used margin" (backing the position)
                // The unfilled portion needs margin reserved for potential future execution
                let (_, unfilled_opening) = decompose_fill(unfilled_size, user_pos + fill_size);
                let margin_to_reserve = compute_required_margin(unfilled_opening, limit_price, pair_params);
                user_state.reserved_margin += margin_to_reserve;

                // Store limit order as GTC
                let order = Order {
                    user_id,
                    size: unfilled_size,
                    reduce_only,
                };
                let order_id = generate_order_id();
                let created_at = current_timestamp();

                if unfilled_size > Dec::ZERO {
                    // Buy order: store with inverted price for descending iteration
                    let key = (pair_id, MAX_PRICE - limit_price, created_at, order_id);
                    BUY_ORDERS.save(key, order);
                } else {
                    // Sell order: store with normal price for ascending iteration
                    let key = (pair_id, limit_price, created_at, order_id);
                    SELL_ORDERS.save(key, order);
                }
            },
        }
    }
}

/// Execute a fill, updating positions and settling PnL for any closing portion.
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

    // Settle PnL for closing portion
    if closing_size != Dec::ZERO {
        if let Some(pos) = position {
            let pnl = compute_pnl_to_realize(pos, closing_size, exec_price);
            settle_pnl(state, user_state, pnl);

            // Update cost_basis proportionally
            let close_ratio = abs(closing_size) / abs(pos.size);
            pos.cost_basis = floor(pos.cost_basis * (1 - close_ratio));
        }
    }

    // Update position size
    if let Some(pos) = position {
        pos.size += fill_size;

        // Add to cost_basis for opening portion
        if opening_size != Dec::ZERO {
            pos.cost_basis += floor(abs(opening_size) * exec_price);
        }

        // Remove position if fully closed
        if pos.size == Dec::ZERO {
            user_state.positions.remove(&pair_id);
        }
    } else if opening_size != Dec::ZERO {
        // Create new position
        user_state.positions.insert(pair_id, Position {
            size: fill_size,
            cost_basis: floor(abs(opening_size) * exec_price),
        });
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
/// - entry_value = proportional cost_basis for the closed portion
/// - exit_value = |closing_size| * exec_price
///
/// TODO: This is a placeholder. Full implementation depends on:
/// - Funding fee accumulation
/// - Precise cost basis tracking
fn compute_pnl_to_realize(
    position: &Position,
    closing_size: Dec,  // Same sign as the order (positive for buys, negative for sells)
    exec_price: Udec,
) -> Dec {
    if closing_size == Dec::ZERO || position.size == Dec::ZERO {
        return Dec::ZERO;
    }

    // Proportion of position being closed
    let close_ratio = abs(closing_size) / abs(position.size);

    // Entry value (proportional cost basis)
    let entry_value = position.cost_basis * close_ratio;

    // Exit value
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
        state.vault_balance = state.vault_balance.saturating_sub(amount);
        user_state.margin += amount;
    } else if pnl < Dec::ZERO {
        // User loses: transfer from user to vault
        let loss = floor(-pnl);
        let user_pays = min(loss, user_state.margin);
        // Bad debt (loss - user_pays) is absorbed by the vault - they simply
        // don't receive payment for it. Proper liquidation should prevent this.
        user_state.margin -= user_pays;
        state.vault_balance += user_pays;
    }
    // pnl == 0: no transfer needed
}
```

### Fulfillment of limit orders

At the beginning of each block, validators submit the latest oracle prices. The contract is then triggered to scan the unfilled limit orders in its storage and look for ones that can be filled.

For buy orders, we iterate from the orders with the highest limit price descendingly, until we reach `limit_price < marginal_price`. From this point beyond, no more buy orders can be filled.

For sell orders, we do the opposite: iterate from the lowest limit price ascendingly, until we reach `limit_price > marginal_price`.

Importantly, we execute both order types in an interleaving manner, which is necessary to ensure faireness. Suppose instead we execute all the buy orders first, then all the sell orders. The buy orders would increase the marginal price, allowing sell orders to execute at higher prices. This favors the sellers and disfavors the buyers. Instead, we go through each side of the order book simultaneously, and pick the order that was created earlier, respecting the **price-time priority**.

```rust
fn fulfill_limit_orders_for_pair(
    state: &mut State,
    pair_id: PairId,
    oracle_price: Udec,
    pair_state: &mut PairState,
    pair_params: &PairParams,
) {
    let mut skew = pair_state.long_oi + pair_state.short_oi;

    // Get iterators for both queues (sorted by price-time within each)
    let mut buy_iter = BUY_ORDERS.prefix(pair_id).range(..).peekable();
    let mut sell_iter = SELL_ORDERS.prefix(pair_id).range(..).peekable();

    loop {
        // Compute marginal price at current skew
        let marginal_price = compute_marginal_price(oracle_price, skew, pair_params);

        // Check if each side's head order passes the cutoff
        let buy_fillable = buy_iter.peek().map_or(false, |(key, _)| {
            let limit_price = MAX_PRICE - key.inverted_limit_price;
            limit_price >= marginal_price
        });

        let sell_fillable = sell_iter.peek().map_or(false, |(key, _)| {
            key.limit_price <= marginal_price
        });

        // Determine which side to process
        let process_buy = match (buy_fillable, sell_fillable) {
            (false, false) => break,  // Neither side can make progress
            (true, false) => true,    // Only buys fillable
            (false, true) => false,   // Only sells fillable
            (true, true) => {
                // Both fillable: pick older timestamp (buy wins ties)
                let buy_ts = buy_iter.peek().unwrap().0.created_at;
                let sell_ts = sell_iter.peek().unwrap().0.created_at;
                buy_ts <= sell_ts
            }
        };

        if process_buy {
            try_fill_buy_order(&mut buy_iter, pair_id, state, oracle_price, &mut skew, pair_state, pair_params);
        } else {
            try_fill_sell_order(&mut sell_iter, pair_id, state, oracle_price, &mut skew, pair_state, pair_params);
        }
    }
}

/// Attempts to fill the next buy order from the iterator.
/// The caller has already verified that this order passes the marginal price cutoff.
/// Always advances the iterator.
///
/// NOTE: No margin check is performed here. Margin was reserved when the order
/// was placed. Upon fill, we release the reserved margin (it converts to "used
/// margin" backing the new position).
fn try_fill_buy_order(
    iter: &mut Peekable<impl Iterator<Item = (BuyOrderKey, Order)>>,
    pair_id: PairId,
    state: &mut State,
    oracle_price: Udec,
    skew: &mut Dec,
    pair_state: &mut PairState,
    pair_params: &PairParams,
) {
    // Advance the iterator
    let (key, order) = iter.next().unwrap();

    // Recover limit_price from inverted storage
    let limit_price = MAX_PRICE - key.inverted_limit_price;

    // Compute actual execution price for this order's size
    let exec_price = compute_exec_price(oracle_price, *skew, order.size, pair_params);

    // Check price constraint
    if exec_price > limit_price {
        // Order is too large to fill at current skew; skip
        // (smaller orders with lower prices might still be fillable)
        return;
    }

    // Load user state and check OI constraint
    let mut user_state = load_user_state(order.user_id);
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let (closing_size, opening_size) = decompose_fill(order.size, user_pos);

    let max_opening = compute_max_opening_from_oi(opening_size, pair_state, pair_params.max_abs_oi);
    let oi_violated = max_opening < opening_size;

    if oi_violated {
        if order.reduce_only {
            // Execute only closing portion
            if closing_size == Dec::ZERO {
                return;  // Nothing to fill
            }

            let fill_size = closing_size;
            let fill_exec_price = compute_exec_price(oracle_price, *skew, fill_size, pair_params);

            if fill_exec_price > limit_price {
                return;  // Still exceeds limit
            }

            execute_fill(state, pair_state, &mut user_state, pair_id, fill_size, fill_exec_price);
            *skew += fill_size;

            // Release reserved margin for the filled portion (closing only, so no opening margin)
            // The unfilled opening portion's margin stays reserved
            // (No change to reserved_margin here since we only filled closing portion)

            // Commit changes
            save_user_state(user_state);

            // Update order: reduce size or remove if fully filled
            let remaining = order.size - fill_size;
            if remaining == Dec::ZERO {
                BUY_ORDERS.remove(key);
            } else {
                BUY_ORDERS.save(key, Order { size: remaining, ..order });
            }
        } else {
            // All-or-nothing: skip this order
            return;
        }
    } else {
        // Fill entire order
        execute_fill(state, pair_state, &mut user_state, pair_id, order.size, exec_price);
        *skew += order.size;

        // Release reserved margin for this order
        // The margin now becomes "used margin" backing the position
        let reserved_for_order = compute_required_margin(opening_size, limit_price, pair_params);
        user_state.reserved_margin = user_state.reserved_margin.saturating_sub(reserved_for_order);

        // Commit changes
        save_user_state(user_state);

        // Remove filled order
        BUY_ORDERS.remove(key);
    }
}

/// Attempts to fill the next sell order from the iterator.
/// The caller has already verified that this order passes the marginal price cutoff.
/// Always advances the iterator.
///
/// NOTE: No margin check is performed here. Margin was reserved when the order
/// was placed. Upon fill, we release the reserved margin (it converts to "used
/// margin" backing the new position).
fn try_fill_sell_order(
    iter: &mut Peekable<impl Iterator<Item = (SellOrderKey, Order)>>,
    pair_id: PairId,
    state: &mut State,
    oracle_price: Udec,
    skew: &mut Dec,
    pair_state: &mut PairState,
    pair_params: &PairParams,
) {
    // Advance the iterator
    let (key, order) = iter.next().unwrap();

    let limit_price = key.limit_price;

    // Compute actual execution price for this order's size (negative for sells)
    let exec_price = compute_exec_price(oracle_price, *skew, order.size, pair_params);

    // Check price constraint (for sells: exec_price must be >= limit_price)
    if exec_price < limit_price {
        // Order is too large to fill at current skew; skip
        return;
    }

    // Load user state and check OI constraint
    let mut user_state = load_user_state(order.user_id);
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let (closing_size, opening_size) = decompose_fill(order.size, user_pos);

    let max_opening = compute_max_opening_from_oi(opening_size, pair_state, pair_params.max_abs_oi);
    let oi_violated = max_opening > opening_size;  // Note: reversed for sells (negative)

    if oi_violated {
        if order.reduce_only {
            // Execute only closing portion
            if closing_size == Dec::ZERO {
                return;  // Nothing to fill
            }

            let fill_size = closing_size;
            let fill_exec_price = compute_exec_price(oracle_price, *skew, fill_size, pair_params);

            if fill_exec_price < limit_price {
                return;  // Still below limit
            }

            execute_fill(state, pair_state, &mut user_state, pair_id, fill_size, fill_exec_price);
            *skew += fill_size;

            // Release reserved margin for the filled portion (closing only, so no opening margin)
            // The unfilled opening portion's margin stays reserved
            // (No change to reserved_margin here since we only filled closing portion)

            // Commit changes
            save_user_state(user_state);

            // Update order
            let remaining = order.size - fill_size;
            if remaining == Dec::ZERO {
                SELL_ORDERS.remove(key);
            } else {
                SELL_ORDERS.save(key, Order { size: remaining, ..order });
            }
        } else {
            // All-or-nothing: skip this order
            return;
        }
    } else {
        // Fill entire order
        execute_fill(state, pair_state, &mut user_state, pair_id, order.size, exec_price);
        *skew += order.size;

        // Release reserved margin for this order
        // The margin now becomes "used margin" backing the position
        let reserved_for_order = compute_required_margin(opening_size, limit_price, pair_params);
        user_state.reserved_margin = user_state.reserved_margin.saturating_sub(reserved_for_order);

        // Commit changes
        save_user_state(user_state);

        // Remove filled order
        SELL_ORDERS.remove(key);
    }
}
```

### Funding fee

> TODO

### PnL and margin requirement

#### User position PnL

> TODO

#### Vault PnL

Since the vault takes on counterparty positions of every trader, its PnL should be the reverse of the total PnL of all traders combined. However, it's not feasible to loop over all trader positions and sum them up. Instead, we calculate based on global accumulator values.

> TODO

### Liquidation and deleveraging

> TODO

## Out-of-scope features

The following are out-of-scope for now, but will be added in the future:

- **Isolated margin**
- **TP/SL**: automatically close the position is PnL reaches an upper or lower threshold.
