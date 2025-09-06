use grug::{PrimaryKey, RawKey, StdError, StdResult};

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
    /// If the order is not fully filled in the first auction, the order is
    /// canceled, and the remaining portion refunded to the user.
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
