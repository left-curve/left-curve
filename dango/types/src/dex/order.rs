use grug::{
    Addr, MathResult, Number, NumberConst, Udec128, Udec128_6, Udec128_24, Uint64, Uint128,
};

/// Numerical identifier of an order (limit or market).
///
/// ## Notes
///
/// ### Storage layout
///
/// For SELL orders, we count order IDs from 0 up; for BUY orders, from
/// `u64::MAX` down.
///
/// The reason for this, is we need orders to be sorted by **price-time priority**
/// in the contract's storage. Meaning, orders with the better prices come first;
/// for those with the same price, the older ones come first. We achieve this by
/// using the following storage key for orders:
///
/// ```plain
/// direction | price | order_id
/// ```
///
/// - For bids, we iterate all orders prefixed with `Direction::Bid`, _ascendingly_.
/// - For asks, we iterate all orders prefixed with `Direction::Ask`, _descendingly_.
///
/// In each case, price-time priority is respected.
///
/// See `dango_dex::state::LimitOrderKey` for details.
///
/// Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which is a
/// safe assumption. If we accept 1 million orders per second, it would take
/// ~300,000 years to reach `u64::MAX / 2`.
///
/// ### Serialization
///
/// JSON uses IEEE-754 64-bit floating point numbers to represent numbers, which
/// can only represent integers up to `2^53 - 1` without losing precision. For
/// example,
///
/// ```javascript
/// JSON.stringify({ number: 9007199254740993 })
/// ```
///
/// returns:
///
/// ```json
/// '{"number":9007199254740992}'
/// ```
///
/// The value is off by 1, because `9007199254740993` is bigger than `2^53 - 1`
/// and thus can't be represented without losing precision.
///
/// Since order IDs for asks are counted from top down, they necessarily exceed
/// `2^53 - 1`. To accurately represent these order IDs, instead of `u64`, we
/// use `grug::Uint64`, which is serialized as JSON strings.
pub type OrderId = Uint64;

/// An identifier type that is extended to represent both real orders from users
/// and virtual orders from the passive pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedOrderId {
    /// An limit or market order created by a user.
    User(OrderId),
    /// A virtual limit order created by the passive pool.
    Passive(OrderId),
}

#[grug::derive(Serde)]
pub enum OrderKind {
    Limit,
    Market,
    Passive,
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub enum Order {
    Limit(LimitOrder),
    Market(MarketOrder),
    Passive(PassiveOrder),
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct LimitOrder {
    pub user: Addr,
    /// The order's identifier.
    pub id: OrderId,
    /// The order's limit price, measured in quote asset per base asset.
    pub price: Udec128_24,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Udec128_6,
    /// The block height at which the order was submitted.
    pub created_at_block_height: u64,
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
    pub remaining: Udec128_6,
    /// Max slippage percentage.
    pub max_slippage: Udec128,
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct PassiveOrder {
    pub id: OrderId,
    pub price: Udec128_24,
    pub amount: Uint128,
    pub remaining: Udec128_6,
}

pub trait OrderTrait {
    /// Return the order's kind.
    fn kind(&self) -> OrderKind;

    /// Return the order's _extended_ identifier.
    fn extended_id(&self) -> ExtendedOrderId;

    /// Return the order's ID and the user who created it. `None` for passive orders.
    fn id_and_user(&self) -> Option<(OrderId, Addr)>;

    /// Return the user who created the order. `None` for passive orders.
    fn user(&self) -> Option<Addr> {
        self.id_and_user().map(|(_, user)| user)
    }

    /// Return the block height at which a limit order was created.
    /// `None` for market or passive orders.
    fn created_at_block_height(&self) -> Option<u64>;

    /// Return the order's remaining amount as an immutable reference.
    fn remaining(&self) -> &Udec128_6;

    /// Return the order's remaining amount as a mutable reference.
    fn remaining_mut(&mut self) -> &mut Udec128_6;

    /// Subtract a given amount from the order's remaining amount.
    fn fill(&mut self, amount: Udec128_6) -> MathResult<()> {
        self.remaining_mut().checked_sub_assign(amount)
    }

    /// Set the order's remaining amount to zero.
    /// Return the remaining amount prior to clearing.
    fn clear(&mut self) -> Udec128_6 {
        let remaining = *self.remaining();
        *self.remaining_mut() = Udec128_6::ZERO;
        remaining
    }
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

    fn remaining(&self) -> &Udec128_6 {
        match self {
            Order::Limit(limit_order) => limit_order.remaining(),
            Order::Market(market_order) => market_order.remaining(),
            Order::Passive(passive_order) => passive_order.remaining(),
        }
    }

    fn remaining_mut(&mut self) -> &mut Udec128_6 {
        match self {
            Order::Limit(limit_order) => limit_order.remaining_mut(),
            Order::Market(market_order) => market_order.remaining_mut(),
            Order::Passive(passive_order) => passive_order.remaining_mut(),
        }
    }
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

    fn remaining(&self) -> &Udec128_6 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Udec128_6 {
        &mut self.remaining
    }
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

    fn remaining(&self) -> &Udec128_6 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Udec128_6 {
        &mut self.remaining
    }
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

    fn remaining(&self) -> &Udec128_6 {
        &self.remaining
    }

    fn remaining_mut(&mut self) -> &mut Udec128_6 {
        &mut self.remaining
    }
}
