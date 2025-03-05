use {
    dango_types::oracle::{GuardianSet, GuardianSetIndex, PrecisionlessPrice, PriceSource, PythId},
    grug::{Denom, Map},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");

pub const PRICES: Map<PythId, PrecisionlessPrice> = Map::new("price");
