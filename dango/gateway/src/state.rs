use {
    dango_types::{
        account_factory::UserIndex,
        gateway::{GlobalOutbound, Movement, RateLimit, Remote},
    },
    grug::{Addr, Denom, Item, Map, Uint128},
    std::collections::BTreeMap,
};

// ------------------------- Parameters: set by admin --------------------------

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");

pub const WITHDRAWAL_FEES: Map<(&Denom, Remote), Uint128> = Map::new("withdrawal_fee");

// --------------------- State: updated by user operations ---------------------

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");

/// Current epoch counter, incremented by `cron_execute` each day.
pub const EPOCH: Item<u64> = Item::new("epoch");

/// Snapshotted supply per denom, set at the start of each rate-limit window by
/// `cron_execute`. Used to compute the daily allowance as
/// `supply * rate_limit`.
pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

/// Global sliding window of non-deposit-backed outbound per denom.
/// Each hourly cron rotates the window; the rolling 24h total is cached.
pub const GLOBAL_OUTBOUND: Map<&Denom, GlobalOutbound> = Map::new("global_out");

/// Per-user, per-denom all-time deposit and withdrawal totals. Observational
/// only — not used in rate-limit checks.
pub const USER_MOVEMENTS: Map<(UserIndex, &Denom), Movement> = Map::new("user_mvmt");
