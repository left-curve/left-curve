use {
    dango_types::{
        UsdPrice,
        perps::{Order, OrderId, PairId, PairParam, PairState, Param, State, UserState},
    },
    grug::{Addr, IndexedMap, Item, Map, MultiIndex, Timestamp, UniqueIndex},
    std::collections::BTreeSet,
};

pub const PARAM: Item<Param> = Item::new("param");

pub const STATE: Item<State> = Item::new("state");

pub const PAIR_IDS: Item<BTreeSet<PairId>> = Item::new("pair_ids");

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");

pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");

pub const USER_STATES: Map<Addr, UserState> = Map::new("user_state");

pub const NEXT_ORDER_ID: Item<OrderId> = Item::new("next_order_id");

pub const BIDS: IndexedMap<OrderKey, Order, OrderIndexes> =
    IndexedMap::new("bid", OrderIndexes::new("bid", "bid__id", "bid__user"));

pub const ASKS: IndexedMap<OrderKey, Order, OrderIndexes> =
    IndexedMap::new("ask", OrderIndexes::new("ask", "ask__id", "ask__user"));

pub type OrderKey = (PairId, UsdPrice, Timestamp, OrderId);

#[grug::index_list(OrderKey, Order)]
pub struct OrderIndexes<'a> {
    pub order_id: UniqueIndex<'a, OrderKey, OrderId, Order>,
    pub user: MultiIndex<'a, OrderKey, Addr, Order>,
}

impl OrderIndexes<'static> {
    pub const fn new(
        pk_namespace: &'static str,
        order_id_namespace: &'static str,
        user_namespace: &'static str,
    ) -> Self {
        OrderIndexes {
            order_id: UniqueIndex::new(
                |(_, _, _, order_id), _| *order_id,
                pk_namespace,
                order_id_namespace,
            ),
            user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
        }
    }
}
