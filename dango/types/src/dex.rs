use {
    grug::{Addr, Denom, Prefixer, PrimaryKey, StdError, StdResult, Udec128, Uint128},
    std::{borrow::Cow, collections::BTreeMap},
};

/// Identifier for a trading pair.
pub type PairId = u32;

/// Identifier for an order.
pub type OrderId = u64;

#[grug::derive(Serde, Borsh)]
pub struct Pair {
    /// Denom of the base asset, typically a volatile asset such as BTC.
    pub base_denom: Denom,
    /// Denom of the quote asset, typically a stable asset such as USDC.
    pub quote_denom: Denom,
}

#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl PrimaryKey for OrderSide {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<Cow<[u8]>> {
        match self {
            OrderSide::Buy => vec![Cow::Borrowed(&[0])],
            OrderSide::Sell => vec![Cow::Borrowed(&[1])],
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [0] => Ok(OrderSide::Buy),
            [1] => Ok(OrderSide::Sell),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                "invalid order side! must be 0|1",
            )),
        }
    }
}

impl Prefixer for OrderSide {
    fn raw_prefixes(&self) -> Vec<Cow<[u8]>> {
        self.raw_keys()
    }
}

/// An active order.
///
/// ## Note
///
/// This type doesn't include the trading pair, or whether it's a BUY or SELL
/// order, or the limit price. This is because orders are stored in a mapping:
///
/// > (pair_id, order_side, limit_price) -> Order
///
/// Those info are included in the mapping's key.
///
/// Not to be confused with [`grug::Order`](grug::Order).
#[grug::derive(Borsh)]
pub struct Order {
    pub order_id: OrderId,
    pub maker: Addr,
    pub size: Uint128,
    pub filled: Uint128,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a trading pair of the given base and quote assets.
    ///
    /// Can only be called by the chain owner, and the pair must not already exist.
    CreatePair(Pair),
    /// Submit an order.
    ///
    /// Must attach exactly one coin that is either the base or quote asset of
    /// the trading pair.
    SubmitOrder {
        pair_id: PairId,
        /// The order must execute at this price or better.
        ///
        /// If not given, the order is a market order, which is essentially a
        /// limit BUY order of infinite limit price, or a limit SELL order with
        /// a zero limit price.
        limit_price: Option<Udec128>,
    },
    /// Cancel an active order.
    ///
    /// Can only be called by the order's maker.
    CancelOrder { order_id: OrderId },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query a single trading pair by ID.
    #[returns(Pair)]
    Pair { pair_id: PairId },
    /// Enumerate all trading pairs.
    #[returns(BTreeMap<PairId, Pair>)]
    Pairs {
        start_after: Option<PairId>,
        limit: Option<u32>,
    },
    /// Query a single active order by ID.
    #[returns(OrderResponse)]
    Order { order_id: OrderId },
    /// Enumerate all active orders.
    #[returns(BTreeMap<OrderId, OrderResponse>)]
    Orders {
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde)]
pub struct OrderResponse {
    pub pair_id: PairId,
    pub maker: Addr,
    pub side: OrderSide,
    pub size: Uint128,
    pub filled: Uint128,
    pub limit_price: Option<Udec128>,
}
