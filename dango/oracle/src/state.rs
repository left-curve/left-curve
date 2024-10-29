use {
    dango_types::oracle::{GuardianSetInfo, PythId},
    grug::{Map, Serde},
    pyth_sdk::PriceFeed,
};

pub const GUARDIAN_SETS: Map<u32, GuardianSetInfo> = Map::new("guardian_sets");

pub const PRICE_FEEDS: Map<PythId, PriceFeed, Serde> = Map::new("price_feeds");
