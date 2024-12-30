use {
    grug::{Addr, Denom, PrimaryKey, RawKey, StdError, StdResult, Udec128, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

// ----------------------------------- types -----------------------------------

/// Numerical identifier of an order.
///
/// For SELL orders, we count order IDs from 0 up; for BUY orders, from `u64::MAX`
/// down.
///
/// As such, given our contract storage layout, between two orders of the same
/// price, the older one is matched first. This follows the principle of
/// **price-time priority**.
pub type OrderId = u64;

#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum Direction {
    /// Give away the quote asset, get the base asset; a.k.a. a BUY order.
    Bid,
    /// Give away the base asset, get the quote asset; a.k.a. a SELL order.
    Ask,
}

impl PrimaryKey for Direction {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            Direction::Bid => vec![RawKey::Fixed8([0])],
            Direction::Ask => vec![RawKey::Fixed8([1])],
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [0] => Ok(Direction::Bid),
            [1] => Ok(Direction::Ask),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                "invalid order direction! must be 0|1",
            )),
        }
    }
}

#[grug::derive(Serde)]
pub struct Pair {
    pub base_denom: Denom,
    pub quote_denom: Denom,
}

#[grug::derive(Serde)]
pub struct OrderResponse {
    pub trader: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrdersByPairResponse {
    pub trader: Addr,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrdersByTraderResponse {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
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
    CancelOrders { order_ids: BTreeSet<OrderId> },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query a single active order by ID.
    #[returns(OrderResponse)]
    Order { order_id: OrderId },
    /// Enumerate active orders across all pairs and from traders.
    #[returns(BTreeMap<OrderId, OrderResponse>)]
    Orders {
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders in a single pair from all traders.
    #[returns(BTreeMap<OrderId, OrdersByPairResponse>)]
    OrdersByPair {
        base_denom: Denom,
        quote_denom: Denom,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    /// Enumerate active orders from a single trader across all pairs.
    #[returns(BTreeMap<OrderId, OrdersByTraderResponse>)]
    OrdersByTrader {
        trader: Addr,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
}

// ---------------------------------- events -----------------------------------

// TODO: add events
