use grug::{PrimaryKey, RawKey, StdError, StdResult};

#[grug::derive(Borsh, Serde)]
#[derive(Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
pub enum TimeInForce {
    /// Good-Til-Canceled (GTC): indicates that if the order is not fully filled
    /// in the first auction, its remaining portion is to be persisted in the
    /// order book, and made available for future auctions, where it becomes a
    /// maker order (an order is a taker in its first auction).
    #[serde(rename = "GTC")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "GTC"))]
    GoodTilCanceled,
    /// Immediate-Or-Cancel (IOC): indicates that if the order is not fully
    /// filled in the first auction, it is to be canceled, and the remaining
    /// portion refunded to the user.
    #[serde(rename = "IOC")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "IOC"))]
    ImmediateOrCancel,
}

impl PrimaryKey for TimeInForce {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey<'_>> {
        match self {
            TimeInForce::GoodTilCanceled => vec![RawKey::Fixed8([0])],
            TimeInForce::ImmediateOrCancel => vec![RawKey::Fixed8([1])],
        }
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        match bytes {
            [0] => Ok(TimeInForce::GoodTilCanceled),
            [1] => Ok(TimeInForce::ImmediateOrCancel),
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                "invalid time-in-force! must be 0|1",
            )),
        }
    }
}
