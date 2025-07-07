use {
    crate::{LimitOrder, MarketOrder},
    dango_types::{
        account_factory::Username,
        dex::{Direction, OrderId, PairParams},
    },
    grug::{
        Addr, CoinPair, Counter, Denom, IndexedMap, Map, MultiIndex, Timestamp, Udec128,
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

/// Stores the total trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Udec128> = Map::new("volume");

/// Stores the total trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Udec128> = Map::new("volume_by_user");

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

#[grug::index_list(MarketOrderKey, MarketOrder)]
pub struct MarketOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, MarketOrderKey, OrderId, MarketOrder>,
    pub user: MultiIndex<'a, MarketOrderKey, Addr, MarketOrder>,
}

#[grug::index_list(LimitOrderKey, LimitOrder)]
pub struct LimitOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, LimitOrderKey, OrderId, LimitOrder>,
    pub user: MultiIndex<'a, LimitOrderKey, Addr, LimitOrder>,
}
