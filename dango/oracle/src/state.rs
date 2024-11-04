use {
    dango_types::oracle::{GuardianSetInfo, PriceSourceCollector},
    grug::{Denom, Map},
};

pub const GUARDIAN_SETS: Map<u32, GuardianSetInfo> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSourceCollector> = Map::new("price_source");
