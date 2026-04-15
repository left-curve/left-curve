use {
    dango_types::{
        account_factory::UserIndex,
        gateway::{RateLimit, Remote, UserMovement},
    },
    grug::{Addr, Denom, Item, Map, Uint128},
    std::collections::BTreeMap,
};

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const RATE_LIMITS: Item<BTreeMap<Denom, RateLimit>> = Item::new("rate_limits");

pub const WITHDRAWAL_FEES: Map<(&Denom, Remote), Uint128> = Map::new("withdrawal_fee");

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");

/// Current epoch counter, incremented by `cron_execute` each day.
pub const EPOCH: Item<u64> = Item::new("epoch");

/// Snapshotted supply per denom, set at the start of each rate-limit window by
/// `cron_execute`. Used to compute the daily allowance as
/// `supply * rate_limit`.
pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

/// Global accumulator tracking how much non-deposit-backed outbound has
/// occurred in the current epoch. Only the excess beyond each user's deposit
/// credit is added here. Reset to zero by `cron_execute`.
pub const OUTBOUND: Map<&Denom, Uint128> = Map::new("outbound");

/// Per-user deposit and withdrawal tracking. Keyed by user index.
pub const USER_MOVEMENTS: Map<UserIndex, UserMovement> = Map::new("user_movement");
