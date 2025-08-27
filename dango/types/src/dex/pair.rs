use std::collections::BTreeSet;

use grug::{
    Bounded, Denom, NonZero, Udec128, Udec128_24, ZeroExclusiveOneExclusive,
    ZeroExclusiveOneInclusive, ZeroInclusiveOneExclusive,
};

/// Parameters of a trading pair.
#[grug::derive(Serde, Borsh)]
pub struct PairParams {
    /// Liquidity token denom of the passive liquidity pool.
    pub lp_denom: Denom,
    /// Specifies the pool type (e.g. Xyk or Geometric).
    pub pool_type: PassiveLiquidity,
    /// Price buckets for the liquidity depth chart.
    pub bucket_sizes: BTreeSet<NonZero<Udec128_24>>,
    /// Fee rate for instant swaps in the passive liquidity pool.
    /// For the xyk pool, this also sets the spread of the orders when the
    /// passive liquidity is reflected onto the orderbook.
    pub swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    // TODO: minimum order size
}

#[grug::derive(Serde, Borsh)]
pub enum PassiveLiquidity {
    Xyk(Xyk),
    Geometric(Geometric),
}

#[grug::derive(Serde, Borsh)]
pub struct Xyk {
    /// How far apart each order is placed.
    pub spacing: Udec128,
    /// The portion of reserve that the pool will keep on hand and not use
    /// to place orders.
    ///
    /// This prevents an edge case where a trader makes an extremely large
    /// trade, reducing one side of the pool's liquidity to zero. This would
    /// cause any subsequent liquidity provision to fail with a "division by
    /// zero" error.
    pub reserve_ratio: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// Maximum number of orders to place on each side of the book.
    pub limit: usize,
}

/// Places liquidity around the oracle price in a geometric progression,
/// such that the liquidity assigned to each price point is a fixed ratio of
/// the liquidity remaining to be assigned. Leading to a geometric
/// progression of order sizes. Where the first order has size `1 - ratio`,
/// the second order has size `(1 - ratio) * ratio`, the third order has size
/// `(1 - ratio) * ratio^2`, and so on.
#[grug::derive(Serde, Borsh)]
pub struct Geometric {
    /// How far apart each order is placed.
    pub spacing: Udec128,
    /// The amount of the remaining liquidity to be assigned to each
    /// consecutive order.
    pub ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    /// Maximum number of orders to place on each side of the book.
    pub limit: usize,
}

/// Updates to a trading pair's parameters.
#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}
