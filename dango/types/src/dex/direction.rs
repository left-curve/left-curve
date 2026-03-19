use std::ops::Neg;

/// The direction of a trade: buy or sell.
#[grug::derive(Serde, Borsh)]
#[derive(Copy, grug::PrimaryKey)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "async-graphql", graphql(rename_items = "lowercase"))]
pub enum Direction {
    /// Give away the quote asset, get the base asset; a.k.a. a BUY order.
    Bid,
    /// Give away the base asset, get the quote asset; a.k.a. a SELL order.
    Ask,
}

impl Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Direction::Bid => Direction::Ask,
            Direction::Ask => Direction::Bid,
        }
    }
}
