use {
    crate::amm::FeeRate,
    grug::{CoinPair, Denom, Uint128},
};

/// Identifier of an AMM pool.
pub type PoolId = u32;

// -------------------------------- pool params --------------------------------

/// Parameters of an AMM pool.
#[grug::derive(Serde)]
pub enum PoolParams {
    Xyk(XykParams),
    Concentracted(ConcentratedParams),
}

/// Parameter of a constant product AMM pool (a.k.a. xyk pool).
#[grug::derive(Serde, Borsh)]
pub struct XykParams {
    /// Percentage of swap output that is charged as liquidity fee, paid to
    /// liquidity providers of the pool.
    pub liquidity_fee_rate: FeeRate,
}

/// Parameter of a concentracted liquidity AMM pool (a.k.a. Curve V2 pool).
#[grug::derive(Serde, Borsh)]
pub struct ConcentratedParams {
    // TODO
}

// -------------------------------- pool state ---------------------------------

/// State of an AMM pool.
#[grug::derive(Serde, Borsh)]
pub enum Pool {
    Xyk(XykPool),
    Concentrated(ConcentratedPool),
}

impl Pool {
    pub fn denoms(&self) -> (Denom, Denom) {
        let (coin1, coin2) = match self {
            Pool::Xyk(xyk) => xyk.liquidity.as_ref(),
            Pool::Concentrated(concentrated) => concentrated.liquidity.as_ref(),
        };

        (coin1.denom.clone(), coin2.denom.clone())
    }
}

/// State of a constant product AMM pool (a.k.a. xyk pool).
#[grug::derive(Serde, Borsh)]
pub struct XykPool {
    /// The pool's parameters.
    pub params: XykParams,
    /// The amount of liquidity provided to this pool.
    pub liquidity: CoinPair,
    /// The total amount of liquidity shares outstanding.
    pub shares: Uint128,
}

/// State of a concentrated liquidity AMM pool (a.k.a. Curve V2 pool).
#[grug::derive(Serde, Borsh)]
pub struct ConcentratedPool {
    /// The pool's parameters.
    pub params: ConcentratedParams,
    /// The amount of liquidity provided to this pool.
    pub liquidity: CoinPair,
    /// The total amount of liquidity shares outstanding.
    pub shares: Uint128,
    // TODO: other params...
}
