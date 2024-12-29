use {
    dango_types::orderbook::{Direction, Order, OrderId, Pair},
    grug::{Counter, IndexedMap, Item, Udec128, UniqueIndex},
};

pub const PAIR: Item<Pair> = Item::new("pair");

pub const ORDER_ID: Counter<OrderId> = Counter::new("order_id", 0, 1);

// (direction, price, order_id) => order
//
// Important: the `order_id` bitwise reversed for BUY orders, such that when
// matching orders, the older orders are matched first.
pub const ORDERS: IndexedMap<(Direction, Udec128, OrderId), Order, OrdersIndex> =
    IndexedMap::new("order", OrdersIndex {
        order_id: UniqueIndex::new(|(_, _, order_id), _| *order_id, "order", "order__id"),
    });

#[grug::index_list((Direction, Udec128, OrderId), Order)]
pub struct OrdersIndex<'a> {
    pub order_id: UniqueIndex<'a, (Direction, Udec128, OrderId), OrderId, Order>,
}
