use {
    dango_types::{account_factory::UserIndex, taxman::Config},
    grug::{Item, Map, Timestamp, Udec128_6, Uint128},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_FEE: Item<(Config, Uint128)> = Item::new("withheld_fee");

/// Cumulative trading volume in the spot and perps DEXs of individual users.
///
/// In Dango, this is used to determine a user's trading fee. The higher the
/// volume of the last 30 days, the lower the fee rate.
///
/// To find a user's volume _in the last X days_, find the latest cumulative
/// volume (A), find the cumulative volume from X days ago (B), then subtract
/// A by B.
///
/// The timestamps in this map are rounded down to the nearest day.
/// The volume is in US dollar terms, in humanized unit.
pub const VOLUMES_BY_USER: Map<(UserIndex, Timestamp), Udec128_6> = Map::new("volume__user");
