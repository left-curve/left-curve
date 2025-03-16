use {
    dango_types::oracle::{GuardianSet, GuardianSetIndex, PrecisionlessPrice, PriceSource, PythId},
    grug::{Denom, Map},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");

/// Map from PythId to (price, sequence). The sequence is used on update
/// to ensure that the price is more recent.
pub const PRICES: Map<PythId, (PrecisionlessPrice, u64)> = Map::new("price");
