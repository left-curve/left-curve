use {
    dango_types::oracle::{GuardianSet, PriceSource},
    grug::{Denom, Map},
};

pub const GUARDIAN_SETS: Map<u32, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");
