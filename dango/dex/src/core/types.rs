use {
    dango_types::dex::{OrderId, OrderKind},
    grug::{Addr, MathResult, Number, NumberConst, Udec128, Uint128},
};

/// An identifier type that is extended to represent both real orders from users
/// and virtual orders from the passive pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedOrderId {
    /// An limit or market order created by a user.
    User(u64),
    /// A virtual limit order created by the passive pool.
    Passive(u64),
}

pub trait OrderTrait {
    /// Return the order's kind.
    fn kind(&self) -> OrderKind;

    /// Return the order's _extended_ identifier.
    fn extended_id(&self) -> ExtendedOrderId;

    /// Return order's ID the user who created it. `None` for passive orders.
    fn id_and_user(&self) -> Option<(OrderId, Addr)>;

    /// Return the user who created the order. `None` for passive orders.
    fn user(&self) -> Option<Addr> {
        self.id_and_user().map(|(_, user)| user)
    }

    /// Return the block height at which a limit order was created.
    /// `None` for market or passive orders.
    fn created_at_block_height(&self) -> Option<u64>;

    /// Return the order's remaining amount as an immutable reference.
    fn remaining(&self) -> &Uint128;

    /// Return the order's remaining amount as a mutable reference.
    fn remaining_mut(&mut self) -> &mut Uint128;

    /// Subtract a given amount from the order's remaining amount.
    fn fill(&mut self, amount: Uint128) -> MathResult<()> {
        self.remaining_mut().checked_sub_assign(amount)
    }

    /// Set the order's remaining amount to zero.
    /// Return the remaining amount prior to clearing.
    fn clear(&mut self) -> Uint128 {
        let remaining = *self.remaining();
        *self.remaining_mut() = Uint128::ZERO;
        remaining
    }
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub enum Order {
    Limit(LimitOrder),
    Market(MarketOrder),
    Passive(PassiveOrder),
}

// TODO: consider using a macro to make this impl less verbose.
impl OrderTrait for Order {
    fn kind(&self) -> OrderKind {
        match self {
            Order::Limit(limit_order) => limit_order.kind(),
            Order::Market(market_order) => market_order.kind(),
            Order::Passive(passive_order) => passive_order.kind(),
        }
    }

    fn extended_id(&self) -> ExtendedOrderId {
        match self {
            Order::Limit(limit_order) => limit_order.extended_id(),
            Order::Market(market_order) => market_order.extended_id(),
            Order::Passive(passive_order) => passive_order.extended_id(),
        }
    }

    fn id_and_user(&self) -> Option<(OrderId, Addr)> {
        match self {
            Order::Limit(limit_order) => limit_order.id_and_user(),
            Order::Market(market_order) => market_order.id_and_user(),
            Order::Passive(passive_order) => passive_order.id_and_user(),
        }
    }

    fn created_at_block_height(&self) -> Option<u64> {
        match self {
            Order::Limit(limit_order) => limit_order.created_at_block_height(),
            Order::Market(market_order) => market_order.created_at_block_height(),
            Order::Passive(passive_order) => passive_order.created_at_block_height(),
        }
    }

    fn remaining(&self) -> &Uint128 {
        match self {
            Order::Limit(limit_order) => limit_order.remaining(),
            Order::Market(market_order) => market_order.remaining(),
            Order::Passive(passive_order) => passive_order.remaining(),
        }
    }

    fn remaining_mut(&mut self) -> &mut Uint128 {
        match self {
            Order::Limit(limit_order) => limit_order.remaining_mut(),
            Order::Market(market_order) => market_order.remaining_mut(),
            Order::Passive(passive_order) => passive_order.remaining_mut(),
        }
    }
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct LimitOrder {
    pub user: Addr,
    /// The order's identifier.
    pub id: OrderId,
    /// The order's limit price, measured in quote asset per base asset.
    pub price: Udec128,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Uint128,
    /// The block height at which the order was submitted.
    pub created_at_block_height: u64,
}

impl OrderTrait for LimitOrder {
    fn kind(&self) -> OrderKind {
        OrderKind::Limit
    }

    fn extended_id(&self) -> ExtendedOrderId {
        ExtendedOrderId::User(self.id)
    }

    fn id_and_user(&self) -> Option<(OrderId, Addr)> {
        Some((self.id, self.user))
    }

    fn created_at_block_height(&self) -> Option<u64> {
        Some(self.created_at_block_height)
    }

    fn remaining(&self) -> &Uint128 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Uint128 {
        &mut self.remaining
    }
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct MarketOrder {
    pub user: Addr,
    /// The order's identifier.
    pub id: OrderId,
    /// For BUY orders, the amount of quote asset; for SELL orders, that of the
    /// base asset.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the unit as the
    /// `amount` field.
    pub remaining: Uint128,
    /// Max slippage percentage.
    pub max_slippage: Udec128,
}

impl OrderTrait for MarketOrder {
    fn kind(&self) -> OrderKind {
        OrderKind::Market
    }

    fn extended_id(&self) -> ExtendedOrderId {
        ExtendedOrderId::User(self.id)
    }

    fn id_and_user(&self) -> Option<(OrderId, Addr)> {
        Some((self.id, self.user))
    }

    fn created_at_block_height(&self) -> Option<u64> {
        None
    }

    fn remaining(&self) -> &Uint128 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Uint128 {
        &mut self.remaining
    }
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct PassiveOrder {
    pub id: OrderId,
    pub price: Udec128,
    pub amount: Uint128,
    pub remaining: Uint128,
}

impl OrderTrait for PassiveOrder {
    fn kind(&self) -> OrderKind {
        OrderKind::Passive
    }

    fn extended_id(&self) -> ExtendedOrderId {
        ExtendedOrderId::Passive(self.id)
    }

    fn id_and_user(&self) -> Option<(OrderId, Addr)> {
        None
    }

    fn created_at_block_height(&self) -> Option<u64> {
        None
    }

    fn remaining(&self) -> &Uint128 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Uint128 {
        &mut self.remaining
    }
}
