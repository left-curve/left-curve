# Perpetual futures exchange: specifications

- This is a perpetual futures (perps) exchange that uses the **peer-to-pool model**, similar to e.g. Ostium. A liquidity pool provides quotes (based on data including oracle price, open interest (OI), and the order's size). All orders are executed against the pool, with the pool taking the counterparty position (e.g. if a user opens a long position of 5 BTC, the pool takes the opposite: a short position of 5 BTC). We call the pool the **counterparty pool**. Empirically, traders in aggregate lose money in the long run, which means counterparty pool makes money. This is in contrary to the peer-to-peer model, where users place orders in an order book; a user's order is executed against other users' orders.
- The exchange operates in **one-way mode**. Meaning, e.g., a user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, the position is modified. This is in contrary to **two-way mode**, where a user can have multiple positions in a single asset. When placing an order, the user can choose whether the order will create a new position or modify an existing one.
- To ensure the protocol's solvency and profitability, it's important the market is close to _neutral_, meaning there is roughly the same amount of long and short OI. We incentivize this through two mechanisms, **skew pricing** and **funding fee** (described in respective sections).
- For now, the exchange supports only **cross margin**. Support for isolated margin may be added in a future update. Uniquely, margin is not handled by the exchange smart contract, but the user's account, which is itself a smart contract. The exchange contract would first fulfills an order, then the account contract would backrun it. During the backrun, the account contract checks the user's margin. If not sufficient, an error is thrown to revert all previous operations.

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
    /// Add liquidity to the counterparty vault.
    ///
    /// User must sent a non-zero amount of the settlement currency (defined in
    /// `Params` below) as attachment of this function call.
    Deposit {
        /// Revert if less than this number of share is minted.
        min_shares_to_mint: Option<Uint>,
    },

    /// Request to withdraw funds from the counterparty vault.
    ///
    /// The request will be fulfilled after the cooldown period, defined in the
    /// global parameters.
    Unlock {
        /// The amount of vault shares the user wishes to burn.
        /// Must be no more than the amount of vault shares the user owns.
        shares_to_burn: Uint,
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
        price: PriceOption,
        time_in_force: TimeInForce,
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

enum PriceOption {
    /// Trade at the current price quoted by the counterparty pool, optionally
    /// with a slippage tolerance.
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
    Limit {
        /// The execution price must be equal to or better than this price.
        limit_price: Udec,
    },
}

enum TimeInForce {
    /// Fill the order as much as possible under the constraint of max slippage
    /// (in the case of market orders) or limit price (limit orders).
    /// If not completely unfilled, persist the unfilled portion in contract
    /// storage until either fully filled or canceled.
    ///
    /// TODO: should GTC orders have an expiration time?
    GoodTilCanceled,

    /// Fill the order as much as possible under the constraint of max slippage
    /// or limit price. Cancel the unfilled portion.
    ImmediateOrCancel,

    // TODO: FillOrKill - revert if the order can't be completely filled.
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
    pub max_abs_oi: Udec,

    /// The maximum allowed difference between long and short OIs.
    /// I.e. the following must be satisfied:
    ///
    /// - |pair_state.long_oi - pair_state.short_oi| <= pair_params.max_abs_skew
    pub max_abs_skew: Udec,
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
    /// The amount of vault shares this user owns.
    pub vault_shares: Uint,

    /// The user's positions.
    pub positions: Map<PairId, Position>,

    /// The user's unfilled GoodTilCanceled (GTC) limit orders.
    pub gtc_orders: Vec<Order>,

    /// The user's vault withdrawals that are pending cooldown.
    pub unlocks: Vec<Unlock>,
}

struct Order {
    pub pair_id: PairId,
    pub size: Dec,
    pub price_option: PriceOption,
}

struct Position {
    // TODO
}

struct Unlock {
    /// The amount of settlement currency to be released to the user once
    /// cooldown completes.
    pub amount_to_release: Uint,

    /// The time when cooldown completes.
    pub end_time: Timestamp,
}
```

## Business logic

For simplicity, we assume the settlement asset (defined by `settlement_denom` in `Params`) is USDT.

### Deposit

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT coins to receive newly minted shares, or burn shares to redeem USDT coins.

At any time, the vault's **equity** is defined as the vault's token balance (which reflects the total amount of deposit from liquidity providers and the vault's realized PnL) plus its unrealized PnL:

```rust
fn compute_vault_equity(state: &State, usdt_price: Udec) -> Dec {
    state.vault_balance + (compute_vault_unrealized_pnl() / usdt_price)
}
```

Suppose the vault receives `amount_received` units of USDT from user:

```rust
fn handle_deposit(
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
        (amount_received * state.vault_share_supply / vault_equity).floor()
    } else {
        (amount_received * DEFAULT_SHARES_PER_AMOUNT).floor()
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

### Unlock

Suppose user requests to burn `shares_to_burn` units of shares:

```rust
fn handle_unlock(
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
    let amount_to_release = (vault_equity * shares_to_burn / state.vault_share_supply).floor();

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
    // Once the cooldown period elapses, the contract needs to be triggered to
    // release the fund and remove this unlock. This is trivial and we ignore it
    // in this spec.
    let end_time = current_time + params.vault_cooldown_period;
    user_state.unlocks.push(Unlock { amount_to_release, end_time });
}
```

### Submit order

#### Skew pricing

**Skew** is defined as:

```rust
let skew = pair_state.long_oi + pair_state.short_oi;
```

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
    let premium = min(max(premium, -pair_params.max_abs_premium), pair_params.max_abs_premium);

    // Apply the premium to the oracle price to arrive at the final execution price.
    oracle_price * (1 + premium)
}
```

#### Handling the order

The algorithm for handling `ExecuteMsg::SubmitOrder` finds the maximum fillable amount under multiple constraints:

1. `max_abs_oi` — caps on long and short open interest
2. `max_abs_skew` — cap on the difference between long and short OI
3. Price constraint — `max_slippage` (market) or `limit_price` (limit)

**Key principles:**

- Closing an existing position is always allowed (OI/skew constraints apply only to the "opening" portion)
- `PriceOption` and `TimeInForce` are orthogonal — all combinations are valid

##### Step 1: Decompose Fill into Closing vs Opening

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

##### Step 2: Compute Target Price

The target price is the worst acceptable execution price for the order.

```rust
fn compute_target_price(
    price_option: &PriceOption,
    oracle_price: Udec,
    skew: Dec,
    pair_params: &PairParams,
    is_buy: bool,
) -> Udec {
    match price_option {
        PriceOption::Market { max_slippage } => {
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
        PriceOption::Limit { limit_price } => *limit_price,
    }
}
```

##### Step 3: Max Fillable from OI Constraint (Opening Portion Only)

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

##### Step 4: Max Fillable from Skew Constraint (Opening Portion Only)

```rust
fn compute_max_opening_from_skew(
    opening_size: Dec,
    skew: Dec,
    max_abs_skew: Udec,
) -> Dec {
    // After fill, |skew + fill| <= max_abs_skew
    // Note: closing portion already applied, skew may have changed

    if opening_size > 0 {
        // skew + s <= max_abs_skew
        min(opening_size, max_abs_skew - skew)
    } else if opening_size < 0 {
        // skew + s >= -max_abs_skew
        max(opening_size, -max_abs_skew - skew)
    } else {
        0
    }
}
```

##### Step 5: Max Fillable from Price Constraint (Entire Fill)

This is the complex part due to premium clamping. The execution price formula:

```plain
premium(s) = clamp((skew + s/2) / skew_scale, -M, M)
exec_price(s) = oracle_price * (1 + premium(s))
```

The premium is piecewise linear in `s`:

- **Lower clamped region**: `s <= 2*(-M*K - skew)` → `premium = -M`
- **Unclamped region**: `2*(-M*K - skew) < s < 2*(M*K - skew)` → `premium = (skew + s/2)/K`
- **Upper clamped region**: `s >= 2*(M*K - skew)` → `premium = +M`

```rust
fn compute_max_from_price(
    size: Dec,
    skew: Dec,
    pair_params: &PairParams,
    oracle_price: Udec,
    target_price: Udec,
) -> Dec {
    let K = pair_params.skew_scale;
    let M = pair_params.max_abs_premium;

    // Target premium: exec_price = oracle_price * (1 + premium)
    // For buy: need premium <= target_premium
    // For sell: need premium >= target_premium
    let target_premium = target_price / oracle_price - 1;

    // Clamping boundaries (values of s where clamping kicks in)
    let s_clamp_lower = 2 * (-M * K - skew);  // premium = -M for s <= this
    let s_clamp_upper = 2 * (M * K - skew);   // premium = +M for s >= this

    if size > 0 {
        // BUY: need premium(s) <= target_premium

        // Case 1: target_premium >= M
        // Even the max clamped premium satisfies constraint
        if target_premium >= M {
            return Dec::MAX;
        }

        // Case 2: target_premium < -M
        // Even the min premium (at s=0) doesn't satisfy
        // Wait, at s=0, premium = clamp(skew/K, -M, M)
        // Need to check marginal premium
        let marginal_premium = clamp(skew / K, -M, M);
        if marginal_premium > target_premium {
            return 0;  // Can't fill anything
        }

        // Case 3: In the middle, solve analytically
        // In unclamped region: (skew + s/2) / K <= target_premium
        // s <= 2 * (K * target_premium - skew)
        let s_unclamped = 2 * (K * target_premium - skew);

        if s_unclamped <= 0 {
            return 0;
        }

        // Check if we hit the upper clamp before the price limit
        if s_unclamped <= s_clamp_upper {
            // Price limit is reached in unclamped region
            s_unclamped
        } else {
            // We would enter upper clamped region
            // In that region, premium = M, exec_price = oracle_price * (1 + M)
            if M <= target_premium {
                Dec::MAX  // Clamped price still satisfies
            } else {
                // Can only go up to where clamping starts
                max(0, s_clamp_upper)
            }
        }
    } else {
        // SELL (size < 0): need premium(s) >= target_premium

        if target_premium <= -M {
            return Dec::MIN;  // Even min clamped premium satisfies
        }

        let marginal_premium = clamp(skew / K, -M, M);
        if marginal_premium < target_premium {
            return 0;  // Can't fill anything
        }

        // In unclamped region: (skew + s/2) / K >= target_premium
        // s >= 2 * (K * target_premium - skew)
        let s_unclamped = 2 * (K * target_premium - skew);

        if s_unclamped >= 0 {
            return 0;
        }

        if s_unclamped >= s_clamp_lower {
            s_unclamped
        } else {
            if -M >= target_premium {
                Dec::MIN
            } else {
                min(0, s_clamp_lower)
            }
        }
    }
}
```

##### Step 6: Main Handler

```rust
fn handle_submit_order(
    pair_state: &mut PairState,
    pair_params: &PairParams,
    user_state: &mut UserState,
    pair_id: PairId,
    oracle_price: Udec,
    size: Dec,
    price_option: PriceOption,
    time_in_force: TimeInForce,
) {
    ensure!(size != 0, "nothing to do");

    let skew = pair_state.long_oi + pair_state.short_oi;
    let user_pos = user_state.positions
        .get(&pair_id)
        .map(|p| p.size)
        .unwrap_or(Dec::ZERO);
    let is_buy = size > 0;

    // Decompose into closing and opening portions
    let (closing_size, opening_size) = decompose_fill(size, user_pos);

    // Closing is always allowed; compute max opening from OI/skew constraints
    // (Applied to current state, since closing happens "first" conceptually)
    let skew_after_close = skew + closing_size;

    let max_opening_oi = compute_max_opening_from_oi(opening_size, pair_state, pair_params.max_abs_oi);
    let max_opening_skew = compute_max_opening_from_skew(opening_size, skew_after_close, pair_params.max_abs_skew);

    let max_opening = if is_buy {
        min(max_opening_oi, max_opening_skew)
    } else {
        max(max_opening_oi, max_opening_skew)
    };

    // The maximum fill from OI/skew is closing + constrained opening
    let max_from_oi_skew = closing_size + max_opening;

    // Compute price constraint for the entire fill
    let target_price = compute_target_price(&price_option, oracle_price, skew, pair_params, is_buy);
    let max_from_price = compute_max_from_price(size, skew, pair_params, oracle_price, target_price);

    // Combine all constraints
    let max_fill = if is_buy {
        min(max_from_oi_skew, max_from_price)
    } else {
        max(max_from_oi_skew, max_from_price)
    };

    // Ensure fill has correct sign
    let fill_size = if is_buy {
        max(Dec::ZERO, min(size, max_fill))
    } else {
        min(Dec::ZERO, max(size, max_fill))
    };

    // Execute the fill
    if fill_size != Dec::ZERO {
        let exec_price = compute_exec_price(oracle_price, skew, fill_size, pair_params);
        execute_fill(pair_state, user_state, pair_id, fill_size, exec_price);
    }

    // Handle unfilled portion
    let unfilled_size = size - fill_size;
    if unfilled_size != Dec::ZERO {
        match time_in_force {
            TimeInForce::ImmediateOrCancel => {
                // Discard unfilled portion (no-op)
            }
            TimeInForce::GoodTilCanceled => {
                // Store for later fulfillment
                user_state.gtc_orders.push(Order {
                    pair_id,
                    size: unfilled_size,
                    price_option,
                });
            }
        }
    }
}
```

### Fulfillment of limit orders

At the beginning of each block, validators submit the latest oracle prices. The contract is then triggered to scan the unfilled limited orders in its storage and look for ones that can be filled.

> TODO

### Funding fee

> TODO

### PnL

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
- **Reduce only**: an order that can only reduce the absolute value of a position's size. It can't "flip" the position to the other direction.
- **TP/SL**: automatically close the position is PnL reaches an upper or lower threshold.
