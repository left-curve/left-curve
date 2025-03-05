use {
    anyhow::{bail, ensure},
    grug::{
        Bounded, Denom, PrimaryKey, RawKey, StdError, StdResult, Udec128, Uint128,
        ZeroInclusiveOneExclusive,
    },
    std::{fmt::Display, str::FromStr},
};

/// Numerical identifier of an order.
///
/// For SELL orders, we count order IDs from 0 up; for BUY orders, from `u64::MAX`
/// down.
///
/// As such, given our contract storage layout, between two orders of the same
/// price, the older one is matched first. This follows the principle of
/// **price-time priority**.
///
/// Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which is a
/// safe assumption. If we accept 1 million orders per second, it would take
/// ~300,000 years to reach `u64::MAX / 2`.
pub type OrderId = u64;

/// The direction of a trade: buy or sell.
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum Direction {
    /// Give away the quote asset, get the base asset; a.k.a. a BUY order.
    Bid,
    /// Give away the base asset, get the quote asset; a.k.a. a SELL order.
    Ask,
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
    /// Curve invariant for the passive liquidity pool.
    pub curve_invariant: CurveInvariant,
    /// Fee rate for instant swaps in the passive liquidity pool.
    pub swap_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    // TODO:
    // - orderbook fee rate (either here or as a global parameter)
    // - tick size (necessary or not?)
    // - minimum order size
}

#[grug::derive(Serde, Borsh)]
pub enum CurveInvariant {
    Xyk,
}

impl Display for CurveInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CurveInvariant::Xyk => "xyk",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for CurveInvariant {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        match s {
            "xyk" => Ok(CurveInvariant::Xyk),
            _ => Err(StdError::deserialize::<Self, _>(
                "str",
                "invalid curve type",
            )),
        }
    }
}

/// Updates to a trading pair's parameters.
#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}

#[grug::derive(Serde)]
pub enum SlippageControl {
    /// Minimum amount out. Transaction will fail if the amount out is less than the
    /// specified amount.
    MinimumOut(Uint128),
    /// Maximum amount in. Transaction will fail if the amount in is greater than the
    /// specified amount.
    MaximumIn(Uint128),
    /// Price limit. Transaction will fail if the execution price is greater than the
    /// specified price for a BUY order, or less than the specified price for a SELL order.
    PriceLimit(Udec128),
}

#[grug::derive(Serde)]
pub struct SwapRoute(Vec<(Denom, Denom)>);

impl SwapRoute {
    pub fn new(pairs: Vec<(Denom, Denom)>) -> Self {
        Self(pairs)
    }

    pub fn validate(
        &self,
        direction: &Direction,
        base_denom: &Denom,
        quote_denom: &Denom,
    ) -> anyhow::Result<()> {
        // Route must be non-empty
        if self.0.is_empty() {
            bail!("swap route is empty");
        }

        // Route must contain base and quote denoms
        match direction {
            &Direction::Bid => {
                ensure!(
                    (&self.start().0 == quote_denom || &self.start().1 == quote_denom)
                        && (&self.end().0 == base_denom || &self.end().1 == base_denom),
                    "invalid route"
                );
            },
            &Direction::Ask => {
                ensure!(
                    (&self.start().0 == base_denom || &self.start().1 == base_denom)
                        && (&self.end().0 == quote_denom || &self.end().1 == quote_denom),
                    "invalid route"
                );
            },
        };

        // Route must be a DAG
        let mut visited = std::collections::BTreeSet::<(Denom, Denom)>::new();
        for pair in &self.0 {
            if visited.contains(pair) {
                bail!("swap route contains a cycle");
            }
            visited.insert(pair.clone());
        }

        Ok(())
    }

    pub fn reverse(&self) -> Self {
        let mut pairs = self.0.clone();
        pairs.reverse();
        Self(pairs)
    }

    pub fn start(&self) -> &(Denom, Denom) {
        &self.0[0]
    }

    pub fn end(&self) -> &(Denom, Denom) {
        &self.0[self.0.len() - 1]
    }
}

impl IntoIterator for SwapRoute {
    type IntoIter = std::vec::IntoIter<(Denom, Denom)>;
    type Item = (Denom, Denom);

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
