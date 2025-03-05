use {
    dango_types::dex::CurveInvariant,
    grug::{CoinPair, Number, Uint128},
};

pub trait TradingFunction {
    /// Calculate the value of the trading invariant.
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128>;

    fn normalized_invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128>;
}

impl TradingFunction for CurveInvariant {
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            // k = x * y
            CurveInvariant::Xyk => {
                let first = *reserve.first().amount;
                let second = *reserve.second().amount;
                Ok(first.checked_mul(second)?)
            },
        }
    }

    fn normalized_invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            // sqrt(k)
            CurveInvariant::Xyk => Ok(self.invariant(reserve)?.checked_sqrt()?),
        }
    }
}
