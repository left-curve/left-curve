use {grug::Map, hyperlane_types::ism::ValidatorSet};

pub const VALIDATOR_SETS: Map<u32, ValidatorSet> = Map::new("validator_set");
