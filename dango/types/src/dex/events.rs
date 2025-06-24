use {
    crate::dex::{Direction, OrderId},
    grug::{Addr, Coin, Coins, Denom, Udec128, Uint128},
};

#[grug::derive(Serde)]
#[grug::event("order_submitted")]
pub struct OrderSubmitted {
    pub order_id: OrderId,
    pub user: Addr,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub price: Udec128,
    pub amount: Uint128,
    pub deposit: Coin,
}

#[grug::derive(Serde)]
#[grug::event("order_canceled")]
pub struct OrderCanceled {
    pub order_id: OrderId,
    pub remaining: Uint128,
    pub refund: Coin,
}

#[grug::derive(Serde)]
#[grug::event("orders_matched")]
pub struct OrdersMatched {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub clearing_price: Udec128,
    pub volume: Uint128,
}

#[grug::derive(Serde)]
#[grug::event("order_filled")]
pub struct OrderFilled {
    pub user: Addr,
    pub order_id: OrderId,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    /// The price at which the order was executed.
    pub clearing_price: Udec128,
    /// The amount (measured in base asset) that was filled.
    pub filled: Uint128,
    /// The amount of coins returned to the user.
    pub refund: Coins,
    /// The amount of protocol fee collected.
    pub fee: Option<Coin>,
    /// Whether the order was _completed_ filled and cleared from the book.
    pub cleared: bool,
}

#[grug::derive(Serde)]
#[grug::event("swapped")]
pub struct Swapped {
    pub user: Addr,
    pub input: Coin,
    pub output: Coin,
}
