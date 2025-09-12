use {
    dango_types::{
        account_factory::Username,
        dex::{Direction, Order, OrderId, PairParams, RestingOrderBookState, TimeInForce},
    },
    grug::{
        Addr, CoinPair, Counter, Denom, IndexedMap, Item, Map, MultiIndex, NumberConst, Timestamp,
        Udec128_6, Udec128_24, Uint64, UniqueIndex,
    },
};

pub const PAUSED: Item<bool> = Item::new("paused");

// (base_denom, quote_denom) => params
pub const PAIRS: Map<(&Denom, &Denom), PairParams> = Map::new("pair");

// (base_denom, quote_denom) => coin_pair
pub const RESERVES: Map<(&Denom, &Denom), CoinPair> = Map::new("reserve");

pub const RESTING_ORDER_BOOK: Map<(&Denom, &Denom), RestingOrderBookState> = Map::new("resting");

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("order_id", Uint64::ONE, Uint64::ONE);

pub const ORDERS: IndexedMap<OrderKey, Order, OrderIndex> = IndexedMap::new("order", OrderIndex {
    order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
    user: MultiIndex::new(|_, order| order.user, "order", "order__user"),
    time_in_force: MultiIndex::new(|_, order| order.time_in_force, "order", "order__tif"),
});

/// Stores the liquidity depths for each bucket size. The value is a tuple of (base, quote) depths.
pub const DEPTHS: Map<DepthKey, (Udec128_6, Udec128_6)> = Map::new("depth");

/// Stores the total (cumulative) trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Udec128_6> = Map::new("volume");

/// Stores the total (cumulative) trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Udec128_6> = Map::new("volume_by_user");

/// Storage key for orders.
///
/// ```plain
/// ((base_denom, quote_denom), direction, price, order_id)
/// ```
pub type OrderKey = ((Denom, Denom), Direction, Udec128_24, OrderId);

/// Storage key for liquidity depths.
///
/// ```plain
/// ((base_denom, quote_denom), bucket_size, direction, bucket)
/// ```
pub type DepthKey<'a> = ((&'a Denom, &'a Denom), Udec128_24, Direction, Udec128_24);

#[grug::index_list(OrderKey, Order)]
pub struct OrderIndex<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, Order>,
    pub user: MultiIndex<'a, OrderKey, Addr, Order>,
    pub time_in_force: MultiIndex<'a, OrderKey, TimeInForce, Order>,
}
