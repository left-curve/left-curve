use grug::{
    Bounded, Denom, Udec128, ZeroExclusiveOneExclusive, ZeroExclusiveOneInclusive,
    ZeroInclusiveOneExclusive,
};

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
