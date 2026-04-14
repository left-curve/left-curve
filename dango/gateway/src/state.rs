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

/// Snapshotted supply per denom, set at the start of each rate-limit window by
/// `cron_execute`. Used to compute the daily allowance as
/// `supply * rate_limit`.
pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

/// Accumulator tracking how much of each denom has been withdrawn in the
/// current rate-limit window. Incremented on every `transfer_remote`, reset to
/// zero by `cron_execute`.
pub const OUTBOUND: Map<&Denom, Uint128> = Map::new("outbound");

/// Accumulator tracking how much of each denom has been received from remote
/// chains in the current rate-limit window. Used as a credit (capped at
/// `daily_allowance`) against the outbound rate limit so that round-trips
/// don't consume other users' withdrawal capacity.
pub const INBOUND: Map<&Denom, Uint128> = Map::new("inbound");
