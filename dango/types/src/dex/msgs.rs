use {
    crate::dex::{Direction, OrderId, PairParams, PairUpdate},
    grug::{Addr, CoinPair, Denom, Udec128, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub pairs: Vec<PairUpdate>,
}

#[grug::derive(Serde)]
/// A set of order IDs, either a specific set or all. Used to cancel orders.
pub enum OrderIds {
    Some(BTreeSet<OrderId>),
    All,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create new, or modify the parametes of existing, trading pairs.
    ///
    /// Can only be called by the chain owner.
    BatchUpdatePairs(Vec<PairUpdate>),
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
    /// Submit a new order.
    ///
    /// - For SELL orders, sender must attach `base_denom` of `amount` amount.
    ///
    /// - For BUY orders, sender must attach `quote_denom` of the amount
    ///   calculated as:
    ///
    ///   ```plain
    ///   ceil(amount * price)
    ///   ```
    SubmitOrder {
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        price: Udec128,
    },
    /// Cancel one or more orders by IDs.
    CancelOrders { order_ids: OrderIds },
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
        start_after: Option<PairPageParam>,
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
        start_after: Option<PairPageParam>,
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

/// Pagination parameters of the `QueryMsg::Pairs` query.
#[grug::derive(Serde)]
pub struct PairPageParam {
    pub base_denom: Denom,
    pub quote_denom: Denom,
}

/// Response type of the `QueryMsg::Reserves` query.
#[grug::derive(Serde)]
pub struct ReservesResponse {
    pub pair: PairPageParam,
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
