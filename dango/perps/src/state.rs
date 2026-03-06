use {
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        perps::{Order, OrderId, PairId, PairParam, PairState, Param, State, UserState},
    },
    grug::{Addr, IndexedMap, Item, Map, MultiIndex, Set, Timestamp, UniqueIndex},
    std::collections::BTreeSet,
};

// --------------------------------- constants ---------------------------------

pub const NEXT_ORDER_ID: Item<OrderId> = Item::new("next_order_id");

pub const LAST_VAULT_ORDERS_UPDATE: Item<u64> = Item::new("last_vault_orders_update");

pub const PARAM: Item<Param> = Item::new("param");

pub const STATE: Item<State> = Item::new("state");

pub const PAIR_IDS: Item<BTreeSet<PairId>> = Item::new("pair_ids");

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");

pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");

pub const USER_STATES: IndexedMap<Addr, UserState, UserStateIndexes> =
    IndexedMap::new("us", UserStateIndexes::new("us", "us__unlock"));

/// For a given trading pair, users who have _long_ positions in this pair,
/// indexed by their entry prices.
///
/// Used during auto-deleveraging (ADL) to find the most profitable positions.
pub const LONGS: Set<(PairId, UsdPrice, Addr)> = Set::new("long");

/// For a given trading pair, users who have _short_ positions in this pair,
/// indexed by their entry prices.
///
/// Used during auto-deleveraging (ADL) to find the most profitable positions.
pub const SHORTS: Set<(PairId, UsdPrice, Addr)> = Set::new("short");

pub const BIDS: IndexedMap<OrderKey, Order, OrderIndexes> =
    IndexedMap::new("bid", OrderIndexes::new("bid", "bid__id", "bid__user"));

pub const ASKS: IndexedMap<OrderKey, Order, OrderIndexes> =
    IndexedMap::new("ask", OrderIndexes::new("ask", "ask__id", "ask__user"));

pub const DEPTHS: Map<DepthKey, (Quantity, UsdValue)> = Map::new("depth");

// ----------------------------------- types -----------------------------------

pub type OrderKey = (PairId, UsdPrice, OrderId);

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
                |(_, _, order_id), _| *order_id,
                pk_namespace,
                order_id_namespace,
            ),
            user: MultiIndex::new(|_, order| order.user, pk_namespace, user_namespace),
        }
    }
}

#[grug::index_list(Addr, UserState)]
pub struct UserStateIndexes<'a> {
    /// If the user state has one or more pending unlocks, the earlist ending
    /// time of those unlocks; otherwise, `Timestamp::MAX`.
    pub earliest_unlock_end_time: MultiIndex<'a, Addr, Timestamp, UserState>,
}

impl UserStateIndexes<'static> {
    pub const fn new(pk_namespace: &'static str, idx_namespace: &'static str) -> Self {
        UserStateIndexes {
            earliest_unlock_end_time: MultiIndex::new(
                |_, user_state| {
                    user_state
                        .unlocks
                        .front()
                        .map(|unlock| unlock.end_time)
                        .unwrap_or(Timestamp::MAX)
                },
                pk_namespace,
                idx_namespace,
            ),
        }
    }
}

pub type DepthKey<'a> = (&'a PairId, UsdPrice, bool, UsdPrice);
