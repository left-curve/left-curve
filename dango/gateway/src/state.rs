use {
    dango_types::gateway::{RateLimit, Remote},
    grug::{Addr, Denom, Item, Map, Uint128},
    std::collections::BTreeMap,
};

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");

pub const WITHDRAWAL_FEES: Map<(&Denom, Remote), Uint128> = Map::new("withdrawal_fee");

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");

pub const OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");
