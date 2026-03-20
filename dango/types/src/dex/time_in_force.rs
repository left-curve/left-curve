#[grug::derive(Borsh, Serde)]
#[derive(Copy, PartialOrd, Ord, Hash, grug::PrimaryKey)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
pub enum TimeInForce {
    /// Good-Til-Canceled (GTC): indicates that if the order is not fully filled
    /// in the first auction, its remaining portion is to be persisted in the
    /// order book, and made available for future auctions, where it becomes a
    /// maker order (an order is a taker in its first auction).
    #[primary_key(index = 0)]
    #[serde(rename = "GTC")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "GTC"))]
    GoodTilCanceled,
    /// Immediate-Or-Cancel (IOC): indicates that if the order is not fully
    /// filled in the first auction, it is to be canceled, and the remaining
    /// portion refunded to the user.
    #[primary_key(index = 1)]
    #[serde(rename = "IOC")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "IOC"))]
    ImmediateOrCancel,
}
