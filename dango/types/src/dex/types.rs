use {
    grug::{
        Bounded, Denom, PrimaryKey, RawKey, StdError, StdResult, Udec128, ZeroInclusiveOneExclusive,
    },
    std::fmt::Display,
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
    /// This also sets the spread of the orders when the passive
    /// liquidity is reflected onto the orderbook.
    pub swap_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    // TODO:
    // - orderbook fee rate (either here or as a global parameter)
    // - tick size (necessary or not?)
    // - minimum order size
}

#[grug::derive(Serde, Borsh)]
pub enum CurveInvariant {
    Xyk {
        /// The number of orders from the current price to place on the
        /// bid and ask sides respectively.
        order_depth: u64,
        /// The order spacing for the passive liquidity pool. This is the price
        /// difference between two consecutive orders in when the passive
        /// liquidity is reflected onto the orderbook.
        order_spacing: Udec128,
    },
}

impl Display for CurveInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CurveInvariant::Xyk {
                order_depth,
                order_spacing,
            } => format!("xyk, order_depth: {order_depth}, order_spacing: {order_spacing}"),
        };
        write!(f, "{}", s)
    }
}

// impl FromStr for CurveInvariant {
//     type Err = StdError;

//     fn from_str(s: &str) -> StdResult<Self> {
//         match s {
//             "xyk" => Ok(CurveInvariant::Xyk),
//             _ => Err(StdError::deserialize::<Self, _>(
//                 "str",
//                 "invalid curve type",
//             )),
//         }
//     }
// }

/// Updates to a trading pair's parameters.
#[grug::derive(Serde)]
pub struct PairUpdate {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    pub params: PairParams,
}
