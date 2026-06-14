use {
    dango_math::Uint128,
    dango_primitives::{Addr, Denom},
    dango_storage::Map,
    dango_types::gateway::{PersonalQuota, Remote},
};

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const WITHDRAWAL_FEES: Map<(&Denom, Remote), Uint128> = Map::new("withdrawal_fee");

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");

pub const PERSONAL_QUOTAS: Map<(Addr, &Denom), PersonalQuota> = Map::new("personal_quota");
