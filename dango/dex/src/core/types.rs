use {
    dango_types::dex::OrderKind,
    grug::{Addr, Udec128, Uint128},
};

#[grug::derive(Borsh, Serde)]
pub enum Order {
    Market(MarketOrder),
    Limit(LimitOrder),
}

impl Order {
    pub fn user(&self) -> Addr {
        match self {
            Order::Market(order) => order.user,
            Order::Limit(order) => order.user,
        }
    }

    pub fn kind(&self) -> OrderKind {
        match self {
            Order::Market(_) => OrderKind::Market,
            Order::Limit(_) => OrderKind::Limit,
        }
    }
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct MarketOrder {
    pub user: Addr,
    /// For BUY orders, the amount of quote asset; for SELL orders, that of the
    /// base asset.
    pub amount: Uint128,
    /// Max slippage percentage.
    pub max_slippage: Udec128,
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct LimitOrder {
    pub user: Addr,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Uint128,
    /// The block height at which the order was submitted.
    pub created_at_block_height: u64,
}
