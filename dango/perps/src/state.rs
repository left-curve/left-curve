use {
    dango_types::perps::{PairId, PairParam, PairState, Param, State, UserState},
    grug::{Addr, Item, Map},
};

pub const PARAM: Item<Param> = Item::new("param");

pub const STATE: Item<State> = Item::new("state");

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");

pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");

pub const USER_STATES: Map<Addr, UserState> = Map::new("user_state");

// TODO: BIDS, ASKS
