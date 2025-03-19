use {
    dango_types::oracle::{PrecisionlessPrice, PriceSource},
    grug::{Denom, Map},
    pyth_types::{GuardianSet, GuardianSetIndex, PythId},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");

/// Map from PythId to (price, sequence). The sequence is used on update
/// to ensure that the price is more recent.
pub const PRICES: Map<PythId, (PrecisionlessPrice, u64)> = Map::new("price");
