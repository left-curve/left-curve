use {
    grug::{Addr, Denom, PrimaryKey, RawKey, StdError, StdResult, Udec128, Uint128},
    std::collections::{BTreeMap, BTreeSet},
};

// ----------------------------------- types -----------------------------------

pub type OrderId = u64;

#[grug::derive(Serde, Borsh)]
pub struct Pair {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    // TODO: add protocol fee rate
}

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
                format!("invalid order direction! must be 0|1"),
            )),
        }
    }
}

#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub struct Order {
    pub trader: Addr,
    pub amount: Uint128, // amount measured in the base asset
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrderResponse {
    pub trader: Addr,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

#[grug::derive(Serde)]
pub struct OrdersByTraderResponseItem {
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub pair: Pair,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    SubmitOrder {
        direction: Direction,
        amount: Uint128,
        price: Udec128,
    },
    CancelOrders {
        order_ids: BTreeSet<OrderId>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(Pair)]
    Pair {},
    #[returns(OrderResponse)]
    Order { order_id: OrderId },
    #[returns(BTreeMap<OrderId, OrderResponse>)]
    Orders {
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
    #[returns(BTreeMap<OrderId, OrdersByTraderResponseItem>)]
    OrdersByTrader {
        trader: Addr,
        start_after: Option<OrderId>,
        limit: Option<u32>,
    },
}

// ---------------------------------- events -----------------------------------

// TODO: add events
