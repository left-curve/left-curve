use {
    crate::{
        account_factory::Username,
        dex::{Direction, OrderId, PairParams, PairUpdate, PassiveOrder, RestingOrderBookState},
    },
    grug::{
        Addr, Bounded, Coin, CoinPair, Denom, MaxLength, NonZero, Timestamp, Udec128, Udec128_6,
        Udec128_24, Uint128, UniqueVec, ZeroInclusiveOneExclusive,
    },
    std::collections::{BTreeMap, BTreeSet},
};

/// A series of liquidity pools for performing swaps.
///
/// The route must not contain loops, e.g. asset A -> B -> A. In other words,
/// the pair IDs must be unique.
///
/// Additionally, we enforce a maximum length of 2. This is to prevent the DoS
/// attack of submitting swaps of extremely long routes.
///
/// 2 is a reasonable number, because at launch, all the trading pairs we plan to
/// support comes with USDC as the quote asset. As such, it's possible to go
/// from any one asset to any other in no more than 2 hops. If we plan to support
/// non-USDC quoted pairs, the maximum route length can be adjusted.
pub type SwapRoute = MaxLength<UniqueVec<PairId>, 2>;

/// A request to create a new limit order.
#[grug::derive(Serde)]
pub enum CreateLimitOrderRequest {
    Bid {
        base_denom: Denom,
        quote_denom: Denom,
        amount_quote: NonZero<Uint128>,
        /// The limit price measured _in the quote asset_, i.e. how many units of
        /// quote asset is equal in value to 1 unit of base asset.
        ///
        /// Note: price must be non-zero, otherwise we get "division by zero" error
        /// which causes cronjob to fail.
        price: NonZero<Udec128_24>,
    },
    Ask {
        base_denom: Denom,
        quote_denom: Denom,
        amount_base: NonZero<Uint128>,
        /// The limit price measured _in the quote asset_, i.e. how many units of
        /// quote asset is equal in value to 1 unit of base asset.
        ///
        /// Note: price must be non-zero, otherwise we get "division by zero" error
        /// which causes cronjob to fail.
        price: NonZero<Udec128_24>,
    },
}

/// A request to create a new market order.
/// Market orders are _immediate or cancel_ (IOC), meaning, if there isn't
/// enough liquidity in the resting order book to fully fill the market
/// order under its max slippage, it's filled as much as possible, with the
/// unfilled portion is canceled.
#[grug::derive(Serde)]
pub enum CreateMarketOrderRequest {
    Bid {
        base_denom: Denom,
        quote_denom: Denom,
        amount_quote: NonZero<Uint128>,
        /// The maximum slippage percentage.
        ///
        /// For a market BUY order, suppose the best (lowest) SELL price in the
        ///   resting order book is `p_best`, then the market order's _average
        ///   execution price_ can't be worse than:
        ///
        ///   ```math
        ///   p_best * (1 + max_slippage)
        ///   ```
        max_slippage: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    },
    Ask {
        base_denom: Denom,
        quote_denom: Denom,
        amount_base: NonZero<Uint128>,
        /// The maximum slippage percentage.
        ///
        /// For a market SELL order, suppose the best (highest) BUY price in the
        ///   resting order book is `p_best`, then the market order's _average
        ///   execution price_ can't be worse than:
        ///
        ///   ```math
        ///   p_best * (1 - max_slippage)
        ///   ```
        max_slippage: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    },
}

#[grug::derive(Serde)]
pub enum CancelOrderRequest {
    Some(BTreeSet<OrderId>),
    All,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub pairs: Vec<PairUpdate>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Messages only the chain owner can call.
    Owner(OwnerMsg),
    /// Messages only the contract itself can call.
    Callback(CallbackMsg),
    /// Create or cancel multiple limit orders in one batch.
    BatchUpdateOrders {
        creates_market: Vec<CreateMarketOrderRequest>,
        creates_limit: Vec<CreateLimitOrderRequest>,
        cancels: Option<CancelOrderRequest>,
    },
    /// Provide passive liquidity to a pair. Unbalanced liquidity provision is
    /// equivalent to a swap to reach the pool ratio, followed by a liquidity
    /// provision at pool ratio.
    ProvideLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
    },
    // Withdraw passive liquidity from a pair. Withdrawal is always performed at
    // the pool ratio.
    WithdrawLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
    },
    /// Perform an instant swap directly in the passive liquidity pools, with an
    /// exact amount of input asset.
    ///
    /// User must send exactly one asset, which must be either the base or quote
    /// asset of the first pair in the `route`.
    ///
    /// User may specify a minimum amount of output, for slippage control.
    SwapExactAmountIn {
        route: SwapRoute,
        minimum_output: Option<Uint128>,
    },
    /// Perform an instant swap directly in the passive liqudiity pools, with an
    /// exact amount of output asset.
    ///
    /// User must send exactly one asset, which must be either the base or quote
    /// asset of the first pair in the `route`.
    ///
    /// Slippage control is implied by the input amount. If required input is
    /// less than what user sends, the excess is refunded. Otherwise, if required
    /// input more than what user sends, the swap fails.
    SwapExactAmountOut {
        route: SwapRoute,
        output: NonZero<Coin>,
    },
}

#[grug::derive(Serde)]
pub enum OwnerMsg {
    /// pause or unpause trading.
    SetPaused(bool),
    /// Create new, or modify the parameters of existing, trading pairs.
    BatchUpdatePairs(Vec<PairUpdate>),
    /// Forcibly cancel all orders (limit, market, incoming) and refund the users.
    ForceCancelOrders {},
}

#[grug::derive(Serde)]
pub enum CallbackMsg {
    /// perform the batch auction; called during `cron_execute`
    Auction {},
}

#[grug::derive(Serde)]
pub enum ReplyMsg {
    AfterAuction {},
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Returns whether tracing is paused.
    #[returns(bool)]
    Paused {},
    /// Query the parameters of a single trading pair.
    #[returns(PairParams)]
    Pair {
        base_denom: Denom,
        quote_denom: Denom,
    },
    /// Enumerate all trading pairs and their parameters.
    #[returns(Vec<PairUpdate>)]
    Pairs {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },
    /// Query the passive liquidity pool reserve of a single trading pair,
    #[returns(CoinPair)]
    Reserve {
        base_denom: Denom,
        quote_denom: Denom,
    },
    /// Enumerate all passive liquidity pool reserves.
    #[returns(Vec<ReservesResponse>)]
    Reserves {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },
    /// Query the resting order book state of a pair.
    #[returns(RestingOrderBookState)]
    RestingOrderBookState {
        base_denom: Denom,
        quote_denom: Denom,
    },
    /// Enumerate the resting order book state of all pairs.
    #[returns(Vec<RestingOrderBookStatesResponse>)]
    RestingOrderBookStates {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },
    /// Query a single active order by ID.
    #[returns(OrderResponse)]
    Order { order_id: OrderId },
    /// Enumerate active orders across all pairs and from users.
    #[returns(BTreeMap<OrderId, OrderResponse>)]
    Orders {
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders in a single pair from all users.
    #[returns(BTreeMap<OrderId, OrdersByPairResponse>)]
    OrdersByPair {
        base_denom: Denom,
        quote_denom: Denom,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders from a single user across all pairs.
    #[returns(BTreeMap<OrderId, OrdersByUserResponse>)]
    OrdersByUser {
        user: Addr,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Returns the trading volume of a user address since the specified timestamp.
    #[returns(Udec128)]
    Volume {
        /// The user's address to query trading volume for.
        user: Addr,
        /// The start timestamp to query trading volume for. If not provided,
        /// user's total trading volume will be returned.
        since: Option<Timestamp>,
    },
    /// Returns the trading volume of a username since the specified timestamp.
    #[returns(Udec128)]
    VolumeByUser {
        /// The username to query trading volume for.
        user: Username,
        /// The start timestamp to query trading volume for. If not provided,
        /// username's total trading volume will be returned.
        since: Option<Timestamp>,
    },
    /// Simulate a liquidity provision.
    /// Returns the amount of LP tokens to be minted.
    #[returns(Coin)]
    SimulateProvideLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
        deposit: CoinPair,
    },
    /// Simulate a liquidity withdrawal.
    /// Returns the amount of the two underlying assets to be refunded.
    #[returns(CoinPair)]
    SimulateWithdrawLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
        lp_burn_amount: Uint128,
    },
    /// Simulate a swap with exact input.
    #[returns(Coin)]
    SimulateSwapExactAmountIn { route: SwapRoute, input: Coin },
    /// Simulate a swap with exact output.
    #[returns(Coin)]
    SimulateSwapExactAmountOut {
        route: SwapRoute,
        output: NonZero<Coin>,
    },
    /// Returns the orders generated by the passive liquidity pool.
    #[returns(BTreeMap<Udec128_24, PassiveOrder>)]
    ReflectCurve {
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        /// Up to how many orders to return.
        limit: Option<u32>,
    },
}

/// Identifier of a trading pair. Consists of the base asset and quote asset
/// denominations.
#[grug::derive(Serde)]
#[derive(Hash)]
pub struct PairId {
    pub base_denom: Denom,
    pub quote_denom: Denom,
}

/// Response type of the `QueryMsg::Reserves` query.
#[grug::derive(Serde)]
pub struct ReservesResponse {
    pub pair: PairId,
    pub reserve: CoinPair,
}

/// Response type of the `QueryMsg::RestingOrderBookState` query.
#[grug::derive(Serde)]
pub struct RestingOrderBookStatesResponse {
    pub pair: PairId,
    pub state: RestingOrderBookState,
}

/// Response type of the `QueryMsg::Order` and `Orders` queries.
#[grug::derive(Serde)]
pub struct OrderResponse {
    pub user: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128_24,
    pub amount: Uint128,
    pub remaining: Udec128_6,
}

/// Response type of the `QueryMsg::OrdersByPair` query.
#[grug::derive(Serde)]
pub struct OrdersByPairResponse {
    pub user: Addr,
    pub direction: Direction,
    pub price: Udec128_24,
    pub amount: Uint128,
    pub remaining: Udec128_6,
}

/// Response type of the `QueryMsg::OrdersByUser` query.
#[grug::derive(Serde)]
pub struct OrdersByUserResponse {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128_24,
    pub amount: Uint128,
    pub remaining: Udec128_6,
}
