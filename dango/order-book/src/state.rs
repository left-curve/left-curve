use {
    crate::{ClientOrderId, FillId, LimitOrder, OrderId, PairId, Quantity, UsdPrice, UsdValue},
    grug::{Addr, IndexedMap, Item, Map, MultiIndex, Timestamp, UniqueIndex},
};

// --------------------------------- constants ---------------------------------

pub const NEXT_ORDER_ID: Item<OrderId> = Item::new("next_order_id");

pub const NEXT_FILL_ID: Item<FillId> = Item::new("next_fill_id");

/// Buy orders.
pub const BIDS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> = IndexedMap::new(
    "bid",
    OrderIndexes::new("bid", "bid__id", "bid__user", "bid__cid"),
);

/// Sell orders.
pub const ASKS: IndexedMap<OrderKey, LimitOrder, OrderIndexes> = IndexedMap::new(
    "ask",
    OrderIndexes::new("ask", "ask__id", "ask__user", "ask__cid"),
);

/// Liquidity depths of the order book.
pub const DEPTHS: Map<DepthKey, (Quantity, UsdValue)> = Map::new("depth");

/// Cumulative trading volume per user, bucketed by day.
/// Key: (user, day_timestamp). Value: lifetime cumulative USD notional.
pub const VOLUMES: Map<(Addr, Timestamp), UsdValue> = Map::new("vol");

// ----------------------------------- types -----------------------------------

pub type OrderKey = (PairId, UsdPrice, OrderId);

#[grug::index_list(OrderKey, LimitOrder)]
pub struct OrderIndexes<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, LimitOrder>,

    /// Lets a trader cancel an order in the same block it was submitted, by
    /// the caller-assigned `client_order_id`. The index function returns at
    /// most one key — empty `Vec` for orders submitted without a
    /// `client_order_id`. Uniqueness is per-sender, enforced by
    /// `UniqueIndex` (returns `StdError::duplicate_data` on collision).
    pub client_order_id: UniqueIndex<'a, OrderKey, (Addr, ClientOrderId), LimitOrder>,

    pub user: MultiIndex<'a, OrderKey, Addr, LimitOrder>,
}

impl OrderIndexes<'static> {
    pub const fn new(
        pk_namespace: &'static str,
        order_id_namespace: &'static str,
        user_namespace: &'static str,
        client_order_id_namespace: &'static str,
    ) -> Self {
        OrderIndexes {
            order_id: UniqueIndex::new(
                |(_, _, order_id), _| *order_id,
                pk_namespace,
                order_id_namespace,
            ),
            client_order_id: UniqueIndex::new2(
                |_, order| match order.client_order_id {
                    Some(cid) => vec![(order.user, cid)],
                    None => Vec::new(),
                },
                pk_namespace,
                client_order_id_namespace,
            ),
            user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
        }
    }
}

pub type DepthKey<'a> = (&'a PairId, UsdPrice, bool, UsdPrice);
