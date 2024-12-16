use {
    dango_types::dex::{Order, OrderId, OrderSide, Pair, PairId},
    grug::{Addr, Counter, Denom, IndexedMap, MultiIndex, Udec128, UniqueIndex},
};

pub const NEXT_PAIR_ID: Counter<PairId> = Counter::new("next_pair_id", 1, 1);

pub const PAIRS: IndexedMap<PairId, Pair, PairIndexes> = IndexedMap::new("pair", PairIndexes {
    denoms: UniqueIndex::new(
        |_, pair| (pair.base_denom.clone(), pair.quote_denom.clone()),
        "pair",
        "pair__denoms",
    ),
});

pub const NEXT_ORDER_ID: Counter<OrderId> = Counter::new("next_order_id", 1, 1);

// (pair_id, order_side, limit_price) -> Order
//
// For market orders, limit price is set to `Udec128::MAX` for BUY orders, or
// zero for SELL orders.
pub const ORDERS: IndexedMap<(PairId, OrderSide, Udec128), Order, OrderIndexes> =
    IndexedMap::new("order", OrderIndexes {
        order_id: UniqueIndex::new(|_, order| order.order_id, "order", "order__id"),
        maker: MultiIndex::new(|_, order| order.maker, "order", "order__maker"),
    });

#[grug::index_list(PairId, Pair)]
pub struct PairIndexes<'a> {
    pub denoms: UniqueIndex<'a, PairId, (Denom, Denom), Pair>,
}

#[grug::index_list((PairId, OrderSide, Udec128), Order)]
pub struct OrderIndexes<'a> {
    pub order_id: UniqueIndex<'a, (PairId, OrderSide, Udec128), OrderId, Order>,
    pub maker: MultiIndex<'a, (PairId, OrderSide, Udec128), Addr, Order>,
}
