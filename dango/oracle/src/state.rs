use {
    dango_types::oracle::{GuardianSet, GuardianSetIndex, PriceSource},
    grug::{Denom, Map},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");
