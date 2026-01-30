# Perpetual futures exchange: specifications

Overview:

- This is a perpetual futures (perps) exchange that uses the **peer-to-pool model**, similar to e.g. Ostium. A liquidity pool provides quotes (based on data including oracle price, open interest (OI), and the order's size). All orders are executed against the pool, with the pool taking the counterparty position (e.g. if a user opens a long position of 5 BTC, the pool takes the opposite: a short position of 5 BTC). We call the pool the **counterparty pool**. Empirically, traders in aggregate lose money in the long run, which means counterparty pool makes money. This is in contrary to the peer-to-peer model, where users place orders in an order book; a user's order is executed against other users' orders.
- The exchange operates in **one-way mode**. Meaning, e.g., a user has exactly 1 position for each tradable asset. If an order is fulfilled for a user who already has a position in that asset, the position is modified. This is in contrary to **two-way mode**, where a user can have multiple positions in a single asset. When placing an order, the user can choose whether the order will create a new position or modify an existing one.
- For now, the exchange supports only **cross margin**. Support for isolated margin may be added in a future update.

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

## Business logic

For simplicity, we assume the settlement asset (defined by `settlement_denom` in `Params`) is USDT.

### Deposit

Ownership of liquidity in the vault is tracked by **shares**. Users deposit USDT coins to receive newly minted shares, or burn shares to redeem USDT coins.

At any time, the vault's **equity** is defined as the vault's token balance (which reflects the total amount of deposit from liquidity providers and the vault's realized PnL) plus its unrealized PnL:

```rust
let vault_equity = vault_state.balance + (compute_vault_unrealized_pnl() / usdt_price);
```

Suppose the vault receives `amount_received` units of USDT from user:

```rust
const DEFAULT_SHARES_PER_AMOUNT: Uint = /* can be any reasonable value */;

ensure!(amount_received > 0, "nothing to do");

let shares_to_mint = if vault_share_supply != 0 {
    ensure!(
        vault_equity > 0,
        "vault is in catastrohpic loss! deposit disabled"
        // If the vault has positive shares (i.e. it isn't empty) but zero or
        // negative equity, the protocol is insolvent, and require intervention
        // e.g. a bailout. Disable deposits in this case.
    );

    // Round the number down, to the advantage of the protocol and disadvantage
    // of the user. This is a principle we must follow throughout the codebase.
    (amount_received * vault_state.share_supply / vault_equity).floor()
} else {
    (amount_received * DEFAULT_SHARES_PER_AMOUNT).floor()
};

if let Some(min_shares_to_mint) = min_shares_to_mint {
    ensure!(
        shares_to_min >= min_shares_to_min,
        "to few shares would be minted"
    );
}

vault_state.balance += amount_received;
vault_state.share_supply += shares_to_mint;
```

### Unlock

Suppose user requests to burn `shares_to_burn` units of shares:

```rust
ensure!(shares_to_burn > 0, "nothing to do");
ensure!(user_share_balance >= shares_to_burn, "can't burn more than what you have");

// Similarly to deposit, first compute the vault's equity.
let vault_equity = vault_state.balance + (compute_vault_unrealized_pnl() / usdt_price);

let amount_to_release = (vault_equity * shares_to_burn / vault_state.share_supply).floor();

ensure!(
    vault_state.balance >= amount_to_release,
    "the vault doesn't have sufficient balance to fulfill with this withdrawal"
    // This can happen if the vault has a very positive unrealized PnL.
    // In this case, the liquidity provider must wait until that PnL is realized
    // (i.e. the losing positions from traders are either closed or liquidated)
    // before withdrawing.
);

vault_state.balance -= amount_to_release;
vault_state.share_supply -= shares_to_burn;

// Compute the time when the tokens can be released.
let release_time = current_time + params.vault_cooldown_period;

// Persist this withdrawal request in contract storage.
// When release_time is reached, the tokens are automatically released to the user.
// This can be achieved through a cronjob, which we ignore in this spec.
save_withdraw_request(user, amount_to_release, release_time);
```

### Submit order

To ensure the protocol's solvency and profitability, it's important the market is close to _neutral_, meaning there is roughly the same amount of long and short OI. We incentivize this through two mechanisms, **skew pricing** and **funding fee** (the latter to be explained in a later section).

**Skew** is defined as:

```rust
let skew = pair_state.long_oi - pair_state.short_oi;
```

- If `skew` is positive, it means all traders combined have a net long exposure, and the counterparty vault has a net short exposure. To incentivize traders to go back to neutral, the vault will offer better prices for selling, and worse price for buying.
- If `skew` is negative, it means all traders combined have a net short exposure, and the counterparty vault has a net long exposure. To incentivize traders to go bakc to neutral, the vault will offer better prices for buying, and worse price for selling.
- If `skew` is zero, it means the market is perfectly neutral, and the vault will not bias towards either buying or selling.

Given a skew and a size, the execution price is calculated as:

```rust
fn compute_exec_price(
    oracle_price: Dec128,
    skew: Dec,
    size: Dec,
    pair_params: PairParams,
) -> Dec128 {
    // The skew after the size is fulfilled.
    let skew_after = skew + size;

    // The average skew before and after the size is fulfilled.
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
