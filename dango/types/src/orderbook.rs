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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderKey {
    pub direction: Direction,
    pub price: Udec128,
    pub order_id: OrderId,
}

impl PrimaryKey for OrderKey {
    type Output = (Direction, Udec128, OrderId);
    type Prefix = Direction;
    type Suffix = (Udec128, OrderId);

    const KEY_ELEMS: u8 = 3;

    fn raw_keys(&self) -> Vec<RawKey> {
        let mut keys = self.direction.raw_keys();
        keys.extend(self.price.raw_keys());
        // For BUY orders, we use the bitwise reverse of `order_id` (which equals
        // `u64::MAX - order_id` numerically) such that older orders are filled.
        // first. This follows the _price-time priority_ rule.
        //
        // Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which
        // is a safe assumption. Even if we accept 1 million orders per second,
        // it would take 5.4e+24 years to reach `u64::MAX / 2` which is about
        // 400 trillion times the age of the universe. The Sun will become a red
        // giant and devour Earth in 5 billion years so by then we're all gone.
        keys.push(RawKey::Fixed64(
            match self.direction {
                Direction::Bid => !self.order_id,
                Direction::Ask => self.order_id,
            }
            .to_be_bytes(),
        ));
        keys
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (direction, price, order_id) = <(Direction, Udec128, OrderId)>::from_slice(bytes)?;
        match direction {
            Direction::Bid => Ok((direction, price, !order_id)),
            Direction::Ask => Ok((direction, price, order_id)),
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
