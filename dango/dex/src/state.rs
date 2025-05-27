use {
    dango_types::{
        account_factory::Username,
        dex::{Direction, OrderId, PairParams},
    },
    grug::{
        Addr, CoinPair, Counter, Denom, IndexedMap, Map, MultiIndex, Timestamp, Udec128, Uint128,
        UniqueIndex,
    },
};

// (base_denom, quote_denom) => params
pub const PAIRS: Map<(&Denom, &Denom), PairParams> = Map::new("pair");

// (base_denom, quote_denom) => coin_pair
pub const RESERVES: Map<(&Denom, &Denom), CoinPair> = Map::new("reserve");

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("order_id", 1, 1);

pub const MARKET_ORDERS: IndexedMap<MarketOrderKey, MarketOrder, MarketOrderIndex> =
    IndexedMap::new("market_order", MarketOrderIndex {
        order_id: UniqueIndex::new(
            |(_, _, order_id), _| *order_id,
            "market_order",
            "market_order__id",
        ),
        user: MultiIndex::new(|_, order| order.user, "market_order", "market_order__user"),
    });

pub const LIMIT_ORDERS: IndexedMap<LimitOrderKey, LimitOrder, LimitOrderIndex> =
    IndexedMap::new("order", LimitOrderIndex {
        order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
        user: MultiIndex::new(|_, order| order.user, "order", "order__user"),
    });

pub const INCOMING_ORDERS: Map<(Addr, OrderId), (LimitOrderKey, LimitOrder)> =
    Map::new("incoming_orders");

/// Storage key for market orders.
///
/// ```plain
/// ((base_denom, quote_denom), direction, order_id)
/// ```
pub type MarketOrderKey = ((Denom, Denom), Direction, OrderId);

/// Storage key for limit orders.
///
/// ```plain
/// ((base_denom, quote_denom), direction, price, order_id)
/// ```
pub type LimitOrderKey = ((Denom, Denom), Direction, Udec128, OrderId);

pub enum OrderType {
    /// A limit order that has already existed at the beginning of this block.
    Maker,
    /// A limit order that was received during the current block.
    Incoming,
    /// A market order.
    Market,
}

#[grug::derive(Borsh, Serde)]
pub enum Order {
    Market(MarketOrder),
    Limit(LimitOrder),
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct MarketOrder {
    pub user: Addr,
    /// For BUY orders, the amount of quote asset; for SELL orders, that of the
    /// base asset.
    pub amount: Uint128,
    /// Max slippage percentage.
    pub max_slippage: Udec128,
}

#[grug::index_list(MarketOrderKey, MarketOrder)]
pub struct MarketOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, MarketOrderKey, OrderId, MarketOrder>,
    pub user: MultiIndex<'a, MarketOrderKey, Addr, MarketOrder>,
}

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct LimitOrder {
    pub user: Addr,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Uint128,
    /// The block height at which the order was submitted.
    pub created_at_block_height: u64,
}

#[grug::index_list(LimitOrderKey, LimitOrder)]
pub struct LimitOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, LimitOrderKey, OrderId, LimitOrder>,
    pub user: MultiIndex<'a, LimitOrderKey, Addr, LimitOrder>,
}

/// Stores the total trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Uint128> = Map::new("volume");

/// Stores the total trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Uint128> = Map::new("volume_by_user");
