use {
    crate::dex::{Direction, OrderId, OrderKind, PairId},
    grug::{Addr, Coin, DecCoin, Denom, Udec128_6, Udec128_24, Uint128},
};

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
    pub price: Option<Udec128_24>,
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
    pub remaining: Udec128_6,
    pub refund: DecCoin<6>,
}

#[grug::derive(Serde)]
#[grug::event("limit_orders_matched")]
pub struct LimitOrdersMatched {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub clearing_price: Udec128_24,
    /// Amount matched denominated in the base asset.
    pub volume: Udec128_6,
}

#[grug::derive(Serde)]
#[grug::event("order_filled")]
pub struct OrderFilled {
    pub user: Addr,
    // `None` if the order is from the passive liquidity pool.
    pub id: Option<OrderId>,
    pub kind: OrderKind,
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub direction: Direction,
    pub filled_base: Udec128_6,
    pub filled_quote: Udec128_6,
    pub refund_base: Udec128_6,
    pub refund_quote: Udec128_6,
    pub fee_base: Udec128_6,
    pub fee_quote: Udec128_6,
    /// The price at which the order was executed.
    pub clearing_price: Udec128_24,
    /// Whether the order was _completed_ filled and cleared from the book.
    pub cleared: bool,
}

impl From<&OrderFilled> for PairId {
    fn from(order: &OrderFilled) -> Self {
        PairId {
            base_denom: order.base_denom.to_owned(),
            quote_denom: order.quote_denom.to_owned(),
        }
    }
}

#[grug::derive(Serde)]
#[grug::event("swapped")]
pub struct Swapped {
    pub user: Addr,
    pub input: Coin,
    pub output: Coin,
}

/// An event indicating that the contract has been paused, either manually by
/// the chain owner, or automatically triggered due to an error in `cron_execute`.
/// Under this state, orders can't be created or canceled, and the end-of-block
/// auction is skipped.
#[grug::derive(Serde)]
#[grug::event("paused")]
pub struct Paused {
    /// `None` if paused by the chain owner manually.
    /// `Some` with the error message if triggered by an error.
    pub error: Option<String>,
}

#[grug::derive(Serde)]
#[grug::event("unpaused")]
pub struct Unpaused {}
