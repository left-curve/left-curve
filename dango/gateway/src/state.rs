use {
    dango_types::gateway::{PersonalQuota, RateLimit, Remote},
    grug::{Addr, Denom, Item, Map, Timestamp, Uint128},
    std::collections::BTreeMap,
};

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");

pub const WITHDRAWAL_FEES: Map<(&Denom, Remote), Uint128> = Map::new("withdrawal_fee");

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");

/// Per-denom outbound cap, refreshed by the cron handler to `supply × limit`.
/// Withdraws are rejected when the trailing-24h withdraw volume plus the new
/// request would exceed this cap.
pub const OUTBOUND_LIMITS: Map<&Denom, Uint128> = Map::new("outbound_limit");

/// Cumulative-to-date withdraw volume per `(denom, minute_bucket)`. The
/// trailing-24h sum is computed by subtracting the latest cumulative from
/// the cumulative recorded at-or-before `now − 24h`.
pub const WITHDRAW_VOLUMES: Map<(&Denom, Timestamp), Uint128> = Map::new("withdraw_volume");

pub const PERSONAL_QUOTAS: Map<(Addr, &Denom), PersonalQuota> = Map::new("personal_quota");
