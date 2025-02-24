use {
    dango_types::dex::{Direction, OrderId, PairParams},
    grug::{Addr, Counter, Denom, IndexedMap, Map, MultiIndex, Udec128, Uint128, UniqueIndex},
};

// (base_denom, quote_denom) => params
pub const PAIRS: Map<(&Denom, &Denom), PairParams> = Map::new("pair");

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("order_id", 0, 1);

pub const ORDERS: IndexedMap<OrderKey, Order, OrderIndex> = IndexedMap::new("order", OrderIndex {
    order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
    user: MultiIndex::new(|_, order| order.user, "order", "order__user"),
});

/// Key is the user's address and the order id.
pub const INCOMING_ORDERS: Map<(Addr, OrderId), (OrderKey, Order)> = Map::new("incoming_orders");

/// Type of the keys under which orders are stored in the contract storage.
///
/// This is nested tuple consisting of:
///
/// ```plain
/// ((base_denom, quote_denom), direction, price, order_id)
/// ```
///
/// TODO: ideally we use `&'a Denom` here, but handling lifetime is tricky.
pub type OrderKey = ((Denom, Denom), Direction, Udec128, OrderId);

#[grug::derive(Borsh, Serde)]
#[derive(Copy)]
pub struct Order {
    pub user: Addr,
    /// The order's total size, measured in the _base asset_.
    pub amount: Uint128,
    /// Portion of the order that remains unfilled, measured in the _base asset_.
    pub remaining: Uint128,
}
#[grug::index_list(OrderKey, Order)]
pub struct OrderIndex<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, Order>,
    pub user: MultiIndex<'a, OrderKey, Addr, Order>,
    // TODO: also index orders by pair
}
