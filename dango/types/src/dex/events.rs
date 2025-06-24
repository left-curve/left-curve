use {
    crate::dex::{Direction, OrderId},
    grug::{Addr, Coin, Coins, Denom, Udec128, Uint128},
};

#[grug::derive(Serde)]
pub enum OrderKind {
    Limit,
    Market,
}

#[grug::derive(Serde)]
#[grug::event("order_created")]
pub struct OrderCreated {
    pub user: Addr,
    pub id: OrderId,
    pub kind: OrderKind,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    /// `None` for market orders.
    pub price: Option<Udec128>,
    /// Amount denominated in the base asset for limit orders and market SELL orders.
    /// Amount denominated in the quote asset for market BUY orders.
    pub amount: Uint128,
    pub deposit: Coin,
}

#[grug::derive(Serde)]
#[grug::event("order_canceled")]
pub struct OrderCanceled {
    pub user: Addr,
    pub id: OrderId,
    pub kind: OrderKind,
    /// Amount that remains unfilled at the time of cancelation.
    ///
    /// This can be either denominated in the base or the quote asset, depending
    /// on order type.
    pub remaining: Uint128,
    pub refund: Coin,
}

#[grug::derive(Serde)]
#[grug::event("orders_matched")]
pub struct LimitOrdersMatched {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub clearing_price: Udec128,
    /// Amount matched denominated in the base asset.
    pub volume: Uint128,
}

#[grug::derive(Serde)]
#[grug::event("order_filled")]
pub struct OrderFilled {
    pub user: Addr,
    pub id: OrderId,
    pub kind: OrderKind,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    /// The price at which the order was executed.
    pub clearing_price: Udec128,
    /// The amount that was filled.
    ///
    /// This can be either denominated in the base or the quote asset, depending
    /// on order type.
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
