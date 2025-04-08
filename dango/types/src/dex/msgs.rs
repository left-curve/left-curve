use {
    crate::dex::{Direction, OrderId, PairParams, PairUpdate},
    grug::{Addr, Coin, CoinPair, Denom, MaxLength, Udec128, Uint128, UniqueVec},
    std::collections::{BTreeMap, BTreeSet},
};

/// A request to create a new limit order.
///
/// When creating a new limit order, the trader must send appropriate amount of
/// funds along with the message:
///
/// - For SELL orders, must send `base_denom` of `amount` amount.
///
/// - For BUY orders, must send `quote_denom` of the amount calculated as:
///
///   ```plain
///   ceil(amount * price)
///   ```
#[grug::derive(Serde)]
pub struct CreateLimitOrderRequest {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    /// The amount of _base asset_ to trade.
    ///
    /// The frontend UI may allow user to choose the amount in terms of the
    /// quote asset, and convert it to the base asset amount behind the scene:
    ///
    /// ```plain
    /// base_asset_amount = floor(quote_asset_amount / price)
    /// ```
    pub amount: Uint128,
    /// The limit price measured _in the quote asset_, i.e. how many units of
    /// quote asset is equal in value to 1 unit of base asset.
    pub price: Udec128,
}

/// A set of order IDs, either a specific set or all. Used to cancel orders.
#[grug::derive(Serde)]
pub enum OrderIds {
    Some(BTreeSet<OrderId>),
    All,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub pairs: Vec<PairUpdate>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create new, or modify the parametes of existing, trading pairs.
    ///
    /// Can only be called by the chain owner.
    BatchUpdatePairs(Vec<PairUpdate>),
    /// Create or cancel multiple limit orders in one batch.
    BatchUpdateOrders {
        creates: Vec<CreateLimitOrderRequest>,
        cancels: Option<OrderIds>,
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
        route: MaxLength<UniqueVec<PairId>, 2>,
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
        route: MaxLength<UniqueVec<PairId>, 2>,
        output: Coin,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
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

/// Response type of the `QueryMsg::Order` and `Orders` queries.
#[grug::derive(Serde)]
pub struct OrderResponse {
    pub user: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

/// Response type of the `QueryMsg::OrdersByPair` query.
#[grug::derive(Serde)]
pub struct OrdersByPairResponse {
    pub user: Addr,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

/// Response type of the `QueryMsg::OrdersByUser` query.
#[grug::derive(Serde)]
pub struct OrdersByUserResponse {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}
