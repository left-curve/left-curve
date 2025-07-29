use {
    grug::{
        Bounded, Denom, PrimaryKey, RawKey, StdError, StdResult, Udec128, Uint64,
        ZeroExclusiveOneExclusive, ZeroExclusiveOneInclusive, ZeroInclusiveOneExclusive,
    },
    std::ops::Neg,
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

/// The direction of a trade: buy or sell.
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum Direction {
    /// Give away the quote asset, get the base asset; a.k.a. a BUY order.
    Bid,
    /// Give away the base asset, get the quote asset; a.k.a. a SELL order.
    Ask,
}

impl Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Direction::Bid => Direction::Ask,
            Direction::Ask => Direction::Bid,
        }
    }
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
                "invalid order direction! must be 0|1",
            )),
        }
    }
}

/// Parameters of a trading pair.
#[grug::derive(Serde, Borsh)]
pub struct PairParams {
    /// Liquidity token denom of the passive liquidity pool.
    pub lp_denom: Denom,
    /// Specifies the pool type (e.g. Xyk or Geometric).
    pub pool_type: PassiveLiquidity,
    /// Fee rate for instant swaps in the passive liquidity pool.
    /// For the xyk pool, this also sets the spread of the orders when the
    /// passive liquidity is reflected onto the orderbook.
    pub swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    // TODO: minimum order size
}

#[grug::derive(Serde, Borsh)]
pub enum PassiveLiquidity {
    Xyk {
        /// The order spacing for the passive liquidity pool.
        ///
        /// This is the price difference between two consecutive orders when
        /// the passive liquidity is reflected onto the orderbook.
        order_spacing: Udec128,
        /// The portion of reserve that the pool will keep on hand and not use
        /// to place orders.
        ///
        /// This prevents an edge case where a trader makes an extremely large
        /// trade, reducing one side of the pool's liquidity to zero. This would
        /// cause any subsequent liquidity provision to fail with a "division by
        /// zero" error.
        reserve_ratio: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    },
    /// Places liquidity around the oracle price in a geometric progression,
    /// such that the liquidity assigned to each price point is a fixed ratio of
    /// the liquidity remaining to be assigned. Leading to a geometric
    /// progression of order sizes. Where the first order has size `1 - ratio`,
    /// the second order has size `(1 - ratio) * ratio`, the third order has size
    /// `(1 - ratio) * ratio^2`, and so on.
    Geometric {
        /// The order spacing for the passive liquidity pool.
        ///
        /// This is the price difference between two consecutive orders when
        /// the passive liquidity is reflected onto the orderbook.
        order_spacing: Udec128,
        /// The amount of the remaining liquidity to be assigned to each
        /// consecutive order.
        ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    },
}

/// Updates to a trading pair's parameters.
#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}
