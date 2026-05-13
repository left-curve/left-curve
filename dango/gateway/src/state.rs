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

/// Per-denom supply snapshot, taken by the cron handler each refresh period
/// and seeded by the first `SetRateLimits` call that adds the denom.
/// Combined with the current `RATE_LIMITS` entry, this yields the outbound
/// cap as `supply × limit`. Snapshotting the supply (rather than the cap)
/// means inbound deposits between cron ticks cannot enlarge the cap.
pub const SUPPLY_SNAPSHOTS: Map<&Denom, Uint128> = Map::new("supply_snapshot");

/// Cumulative-to-date withdraw volume per `(denom, hour_bucket)`. The
/// trailing-24h sum is computed by subtracting the latest cumulative from
/// the cumulative recorded at-or-before `now − 24h`.
pub const WITHDRAW_VOLUMES: Map<(&Denom, Timestamp), Uint128> = Map::new("withdraw_volume");

pub const PERSONAL_QUOTAS: Map<(Addr, &Denom), PersonalQuota> = Map::new("personal_quota");
