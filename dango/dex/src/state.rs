use {
    dango_types::dex::{Direction, OrderId},
    grug::{Addr, Counter, Counters, Denom, IndexedMap, Udec128, Uint128, UniqueIndex},
};

/// The number of new orders that each trading pair has received during the
/// current block.
///
/// At the end of the block, we perform order matching for all pairs that have
/// received new orders.
pub const NEW_ORDER_COUNTS: Counters<(&Denom, &Denom), u32> = Counters::new("order_count", 0, 1);

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("order_id", 0, 1);

pub const ORDERS: IndexedMap<OrderKey, Order, OrderIndex> = IndexedMap::new("order", OrderIndex {
    order_id: UniqueIndex::new(|(_, _, _, order_id), _| *order_id, "order", "order__id"),
});

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

#[grug::derive(Borsh)]
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
    // TODO: also index orders by pair and user
}
