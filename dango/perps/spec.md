# Perpetual futures exchange: specifications

- This is a perpetual futures (perps) exchange that uses the **peer-to-pool model**, similar to e.g. Ostium. A liquidity pool provides quotes (based on data including oracle price, open interest (OI), and the order's size). All orders are executed against the pool, with the pool taking the counterparty position (e.g. if a user opens a long position of 5 BTC, the pool takes the opposite: a short position of 5 BTC). We call the pool the **counterparty pool**. Empirically, traders in aggregate lose money in the long run, which means counterparty pool makes money. This is in contrary to the peer-to-peer model, where users place orders in an order book; a user's order is executed against other users' orders.
- The exchange operates in **one-way mode**. Meaning, e.g., a user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, the position is modified. This is in contrary to **two-way mode**, where a user can have multiple positions in a single asset. When placing an order, the user can choose whether the order will create a new position or modify an existing one.
- For now, the exchange supports only **cross margin**. Support for isolated margin may be added in a future update.

This spec is divided into three sections:

- _API_ defines the methods available for users to interact with the smart contract.
- _Storage_ defines what data are to be saved in the smart contract's storage.
- _Business logic_ defines how the smart contract should handle user requests through the API.

Code snippets that follow are in Rust pseudo-code.

## API

User may interact with the smart contract by dispatching the following execution message:

```rust
enum ExecuteMsg {
    /// Add liquidity to the counterparty vault.
    Deposit { min_shares_to_mint: Option<Uint128> },

    /// Request to withdraw funds from the counterparty vault.
    ///
    /// The request will be fulfilled after the cooldown period, defined in the
    /// global parameters.
    Unlock { shares_to_burn: Uint128 },

    /// Submit an order.
    SubmitOrder {
        // The pair ID can either be numerical or string-like.
        pair_id: PairId,
        direction: Direction,
        price: PriceOption,
        /// The amount of the futures contract, not the amount of the settlement asset.
        amount: Uint128,
        time_in_force: TimeInForce,
    },

    /// Forcibly close all of a user's positions.
    ///
    /// This can happen during a liquidation (callable by anyone), or during
    /// auto-deleveraging (callable only by the administrator).
    ForceClose { user: Addr },
}

enum Direction {
    /// Buy -- increase long exposure, or decrease short exposure.
    Bid,
    /// Sell -- decrease long exposure, or increase short exposure.
    Ask,
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
        max_slippage: Udec128,
    },
    /// The execution price must be equal to or better than the specified.
    Limit(Udec128),
}

enum TimeInForce {
    /// Fill the order as much as possible under the constraint of max slippage
    /// (in the case of market orders) or limit price (limit orders).
    /// If not completely unfilled, persist the unfilled portion in contract
    /// storage until either fully filled or canceled.
    GoodTilCanceled,

    /// Fill the order as much as possible under the constraint of max slippage
    /// or limit price. Cancel the unfilled portion.
    ImmediateOrCancel,
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
    pub skew_scale: Udec128,

    /// The maximum extend to which skew can affect execution price.
    /// The execution price is capped in the range `[1 - max_abs_premium, 1 + max_abs_premium]`.
    /// This prevents an exploit where a trader fabricates a big skew to obtain
    /// an unusually favorable pricing.
    /// See the Mars Protocol hack: <https://x.com/neutron_org/status/2014048218598838459>.
    pub max_abs_premium: Udec128,

    // TODO: max OI, max skew
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
    pub vault_balance: Uint128,

    /// Total supply of the vault's share token.
    pub vault_share_supply: Uint128,
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
    pub long_oi: Uint128,

    /// The sum of the absolute value of the sizes of all short positions.
    ///
    /// This can only be non-negative.
    pub short_oi: Uint128,
}
```

## Business logic

For simplicity, we assume the settlement asset (defined by `settlement_denom` in `Params`) is USDT.

### Counterparty pool

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT coins to receive newly minted shares, or burn shares to redeem USDT coins.

At any time, the vault's **withdrawable amount** of USDT is defined as:

```rust
let vault_withdrawable_balance = vault_state.balance + vault_unrealized_pnl;
```

The calculation of `vault_unrealized_pnl` is explained in a later section.

#### Deposit

Suppose the vault receives `amount_received` units of USDT from user:

```rust
let shares_to_mint = if vault_share_supply != 0 {
    ensure!(
        vault_withdrawable_balance > 0,
        "vault is in catastrohpic loss! deposit disabled"
    );

    floor(amount_received * vault_state.share_supply / vault_withdrawable_balance)
} else {
    floor(amount_received * DEFAULT_SHARES_PER_AMOUNT)
};

vault_state.balance += amount_received;
vault_state.share_supply += shares_to_mint;
```

Notes:

1. `DEFAULT_SHARES_PER_AMOUNT` is a constant of value `1_000_000`.
2. `shares_to_mint` is _floored_, to the advantage of the protocol and disadvantage of user. This is a principle we must follow for all roundings throughout the entire protocol.

The contract should ensure `shares_to_mint` is no less than `min_shares_to_mint` (if specified), then mint the shares to the user.

#### Withdrawing

Suppose user requests to burn `shares_to_burn` units of shares:

```rust
// See note 1
let amount_to_release = floor(vault_withdrawable_balance * shares_to_burn / vault_state.share_supply);

// See note 2
vault_state.balance -= amount_to_release;

// See note 1
vault_state.share_supply -= shares_to_burn;
```

Notes:

1. It should always be true that `vault_state.share_supply` >= `shares_to_burn` > 0. Otherwise, panic.
2. Gracefully bail if this subtraction underflows. This can happen if the vault has a very positive unrealized PnL. The user must wait until the PnL is realized (i.e. the traders who are in loss realizes their losses or are liquidated) before withdrawing.

The contract should add the withdrawal requests to a queue, and automatically fulfills it once the cooldown period has elapsed.

### Order settlement

To ensure the protocol's solvency and profitability, it's important the market is close to _neutral_, meaning there is roughly the same amount of long and short OI. We incentivize this through two mechanisms, **skew pricing** and **funding fee** (the latter to be explained in a later section).

**Skew** is defined as:

```rust
let skew = pair_state.long_oi - pair_state.short_oi;
```

- If `skew` is positive, it means all traders combined have a net long exposure, and the counterparty vault has a net short exposure. To incentivize traders to go back to neutral, the vault will offer better prices for selling, and worse price for buying.
- If `skew` is negative, it means all traders combined have a net short exposure, and the counterparty vault has a net long exposure. To incentivize traders to go bakc to neutral, the vault will offer better prices for buying, and worse price for selling.
- If `skew` is zero, it means the market is perfectly neutral, and the vault will not bias towards either buying or selling.

The `skew_pricing` is determined as:

```rust
fn compute_exec_price(
    oracle_price: Dec128,
    skew: Int128,
    order: &Order,
    pair_params: &PairParams,
) -> Dec128 {
    // The skew if the order is fully executed.
    // Note that both `skew` and `order.size` are signed numbers; positive means
    // long/buy, negative means short/sell.
    let skew_after = skew + order.size;

    // The average skew before and after the order is executed.
    let skew_average = (skew + skew_after) / 2;

    // Compute a premium based on the average skew and skew scaling factor.
    let premium = skew_average / pair_params.skew_scale;

    // Bound the premium between [1 - max_abs_premium, 1 + max_abs_premium].
    let premium = min(max(premium, 1 - pair_params.max_abs_premium), 1 + pair_params.max_abs_premium);

    // Apply the premium to the oracle price to arrive at the final execution price.
    oracle_price * (1 + premium)
}
```

> TODO

#### Out-of-scope features

The following advanced order types are out-of-scope for now, but will be added in the future:

- **Reduce only**: an order that can only reduce the absolute value of a position's size. It can't "flip" the position to the other direction.
- **TP/SL**: automatically close the position is PnL reaches an upper or lower threshold.

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
