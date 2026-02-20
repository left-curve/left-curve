use {
    dango_types::perps::{PairId, PairParam},
    grug::Map,
};

pub const PAIR_PARAMS: Map<&PairId, PairParam> = Map::new("pair_param");
