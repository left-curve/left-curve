use grug::{Addr, MathResult, Number, NumberConst, Udec128_6, Udec128_24, Uint64, Uint128};

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

#[grug::derive(Borsh, Serde)]
#[derive(Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "async-graphql", graphql(rename_items = "snake_case"))]
pub enum TimeInForce {
    /// If the order is not fully filled in the first auction, its remaining
    /// portion is persisted in the order book, and is available to be matched
    /// again in future auctions, where it becomes a maker order (an order is
    /// a taker in its first auction).
    GoodTilCanceled,
    /// If the order is not fully filled in the first auction,  the order is
    /// canceled, and the remaining portion refunded to the user.
    ImmediateOrCancel,
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct Order {
    /// The user who created the order.
    pub user: Addr,
    /// The order's identifier.
    pub id: OrderId,
    /// The order's time-in-force.
    pub time_in_force: TimeInForce,
    /// The order's limit price, measured in quote asset per base asset.
    pub price: Udec128_24,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Udec128_6,
    /// The block height at which the order was submitted. `None` for passive orders.
    pub created_at_block_height: Option<u64>,
}

impl Order {
    /// Subtract a given amount from the order's remaining amount.
    pub fn fill(&mut self, amount: Udec128_6) -> MathResult<()> {
        self.remaining.checked_sub_assign(amount)
    }

    /// Set the order's remaining amount to zero.
    /// Return the remaining amount prior to clearing.
    pub fn clear(&mut self) -> Udec128_6 {
        let remaining = self.remaining;
        self.remaining = Udec128_6::ZERO;
        remaining
    }
}
