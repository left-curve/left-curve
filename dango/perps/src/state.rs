use {
    dango_types::perps::{PairId, PairParam, PairState},
    grug::Map,
};

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");

pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");
