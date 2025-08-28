use {
    dango_types::{
        account_factory::Username,
        dex::{Direction, Order, OrderId, PairParams, RestingOrderBookState},
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

pub const MARKET_ORDERS: Map<(Addr, OrderId), (OrderKey, Order)> = Map::new("market");

pub const LIMIT_ORDERS: IndexedMap<OrderKey, Order, LimitOrderIndex> =
    IndexedMap::new("order", LimitOrderIndex {
        order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
        user: MultiIndex::new(|_, order| order.user, "order", "order__user"),
    });

/// Stores the total trading volume in USD for each account address and timestamp.
pub const VOLUMES: Map<(&Addr, Timestamp), Udec128_6> = Map::new("volume");

/// Stores the total trading volume in USD for each username and timestamp.
pub const VOLUMES_BY_USER: Map<(&Username, Timestamp), Udec128_6> = Map::new("volume_by_user");

/// Storage key for orders, both limit and market.
///
/// - For limit orders, the `price` is the limit price.
/// - For market orders, it is calculated based on the best price available in
///   the resting order book and the order's maximum slippage.
///
/// ```plain
/// ((base_denom, quote_denom), direction, price, order_id)
/// ```
pub type OrderKey = ((Denom, Denom), Direction, Udec128_24, OrderId);

#[grug::index_list(OrderKey, Order)]
pub struct LimitOrderIndex<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, Order>,
    pub user: MultiIndex<'a, OrderKey, Addr, Order>,
}
