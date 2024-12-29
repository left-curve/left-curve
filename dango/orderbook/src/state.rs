use {
    dango_types::orderbook::{Order, OrderId, OrderKey, Pair},
    grug::{Counter, IndexedMap, Item, UniqueIndex},
};

pub const PAIR: Item<Pair> = Item::new("pair");

pub const ORDER_ID: Counter<OrderId> = Counter::new("order_id", 0, 1);

pub const ORDERS: IndexedMap<OrderKey, Order, OrdersIndex> =
    IndexedMap::new("order", OrdersIndex {
        order_id: UniqueIndex::new(
            |OrderKey { order_id, .. }, _| *order_id,
            "order",
            "order__id",
        ),
    });

#[grug::index_list(OrderKey, Order)]
pub struct OrdersIndex<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, Order>,
}
