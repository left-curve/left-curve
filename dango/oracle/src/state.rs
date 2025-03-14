use {
    dango_types::oracle::{PrecisionlessPrice, PriceSource},
    grug::{Denom, Map},
    pyth_types::{GuardianSet, GuardianSetIndex, PythId},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");

pub const PRICES: Map<PythId, PrecisionlessPrice> = Map::new("price");
