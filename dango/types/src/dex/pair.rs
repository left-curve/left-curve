use {
    crate::dex::Price,
    grug::{
        Bounded, Dec, Denom, Duration, NonZero, Udec128, Uint128, ZeroExclusiveOneExclusive,
        ZeroExclusiveOneInclusive, ZeroInclusiveOneExclusive,
    },
    std::collections::BTreeSet,
};

/// Parameters of a trading pair.
#[grug::derive(Serde, Borsh)]
pub struct PairParams {
    /// Liquidity token denom of the passive liquidity pool.
    pub lp_denom: Denom,
    /// Specifies the pool type (e.g. Xyk or Geometric).
    pub pool_type: PassiveLiquidity,
    /// Price buckets for the liquidity depth chart.
    pub bucket_sizes: BTreeSet<NonZero<Price>>,
    /// Fee rate for instant swaps in the passive liquidity pool.
    /// For the xyk pool, this also sets the spread of the orders when the
    /// passive liquidity is reflected onto the orderbook.
    pub swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// Minimum order size, defined _in the base asset_.
    pub min_order_size_base: Uint128,
    /// Minimum order size, defined _in the quote asset_.
    pub min_order_size_quote: Uint128,
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
    /// Avellaneda-Stoikov model parameters.
    pub avellaneda_stoikov_params: AvellanedaStoikovParams,
}

#[grug::derive(Serde, Borsh)]
/// Parameters for the Avellaneda-Stoikov model. This is used to change the spread
/// and mid-price of the reflected orders as the inventory and volatility changes in
/// relation to the oracle price.
pub struct AvellanedaStoikovParams {
    /// A configurable constant that defines the inventory risk aversion.
    pub gamma: Dec<u128, 24>,
    /// The time horizon in seconds over which to
    pub time_horizon: Duration,
    /// Depth slope parameter. This is used in the A-S model assumptions to describe
    /// the probability of getting as a function of the spread from the mid-price by
    /// modeling it as a Poisson process.
    ///
    /// P(spread) = P_A*exp(-k*spread)
    ///
    /// where P_A is the probability of getting an order at the mid-price, and k is the
    /// depth slope parameter.
    ///
    /// The depth slope parameter is used to control the steepness of the depth curve.
    /// A higher k value will result in a more steep depth curve, meaning that the depth
    /// will decrease more rapidly as the spread increases.
    /// A lower k value will result in a more shallow depth curve, meaning that the
    /// depth will decrease more slowly as the spread increases.
    ///
    /// TODO: add link to docs on how to configure k.
    pub k: Dec<u128, 24>,
    /// The half life of the weight of each sample in the volatility estimate.
    ///
    /// The volatility estimate is smoothed using an exponential moving average, where the
    /// volatility estimate is updated as follows:
    ///
    /// vol_estimate_t = alpha * vol_estimate_{t-1}^2 + (1 - alpha) * r_t^2
    ///
    /// where vol_estimate_t is the volatility estimate at time t, and vol_estimate_{t-1} is the
    /// volatility estimate at time t-1 and r_t is the log return at time t.
    ///
    /// alpha is calculated as follows:
    ///
    /// alpha_i = 1 - e^(-ln(2) * dt_i / half_life)
    ///
    /// where dt_i is the time difference between the current and previous sample in milliseconds.
    /// So half_life sets the rate at which the weight of each sample decays.
    pub half_life: Duration,
    /// The target inventory percentage of the base asset.
    pub base_inventory_target_percentage: Bounded<Udec128, ZeroExclusiveOneExclusive>,
}

/// Updates to a trading pair's parameters.
#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}
