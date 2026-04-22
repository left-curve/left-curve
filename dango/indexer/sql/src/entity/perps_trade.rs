#[cfg(feature = "async-graphql")]
use async_graphql::SimpleObject;

/// In-memory representation of a perps `OrderFilled` event for real-time
/// streaming via GraphQL subscriptions. Not persisted to a dedicated table —
/// the raw event data already lives in `perps_events`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PerpsTrade"))]
pub struct PerpsTrade {
    #[cfg_attr(feature = "async-graphql", graphql(name = "orderId"))]
    pub order_id: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "pairId"))]
    pub pair_id: String,

    pub user: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "fillPrice"))]
    pub fill_price: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "fillSize"))]
    pub fill_size: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "closingSize"))]
    pub closing_size: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "openingSize"))]
    pub opening_size: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "realizedPnl"))]
    pub realized_pnl: String,

    pub fee: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "createdAt"))]
    pub created_at: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "blockHeight"))]
    pub block_height: u64,

    #[cfg_attr(feature = "async-graphql", graphql(name = "tradeIdx"))]
    pub trade_idx: u32,

    /// Identifier shared between the two `OrderFilled` events of a single
    /// order-book match. `None` for trades executed before v0.15.0 — fill
    /// IDs were not assigned prior to that release.
    #[cfg_attr(feature = "async-graphql", graphql(name = "fillId"))]
    pub fill_id: Option<String>,

    /// `Some(true)` for the maker side of a match, `Some(false)` for the
    /// taker side. `None` for trades executed before v0.16.0 — the
    /// maker/taker flag was not recorded prior to that release.
    #[cfg_attr(feature = "async-graphql", graphql(name = "isMaker"))]
    pub is_maker: Option<bool>,
}
