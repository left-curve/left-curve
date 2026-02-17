use {
    dango_types::{
        account_factory::UserIndex,
        taxman::{Config, Referee, RefereeStats, Referrer, ShareRatio, UserReferralData},
    },
    grug::{IndexedMap, Item, Map, MultiIndex, Timestamp, Udec128, Udec128_6, Uint128},
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
/// The volume is in USDC microunits, i.e. 1e-6 USDC.
pub const VOLUMES_BY_USER: Map<(UserIndex, Timestamp), Udec128_6> = Map::new("volume__user");

/// Given a referee, find his referrer.
pub const REFEREE_TO_REFERRER: Map<Referee, Referrer> = Map::new("referee_to_referrer");

/// How much of the fee the referrer want to share with the referee.
pub const FEE_SHARE_RATIO: Map<Referrer, ShareRatio> = Map::new("fee_share_ratio");

/// Stores the total (cumulative) data for an user related to the referral program by day.
pub const USER_REFERRAL_DATA: Map<(UserIndex, Timestamp), UserReferralData> =
    Map::new("user_referral_data");

/// Stores the statistics of referees for each referrer.
pub const REFERRER_TO_REFEREE_STATISTICS: IndexedMap<
    (Referrer, Referee),
    RefereeStats,
    ReferrerStatisticsIndex,
> = IndexedMap::new("ref_stats", ReferrerStatisticsIndex {
    register_at: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.registered_at),
        "ref_stats",
        "ref_stats__register_at",
    ),
    volume: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.volume),
        "ref_stats",
        "ref_stats__volume",
    ),
    commission: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.commission_rebounded),
        "ref_stats",
        "ref_stats__commission",
    ),
});

#[grug::index_list((Referrer, Referee), RefereeStats)]
pub struct ReferrerStatisticsIndex<'a> {
    pub register_at: MultiIndex<'a, (Referrer, Referee), (Referrer, Timestamp), RefereeStats>,
    pub volume: MultiIndex<'a, (Referrer, Referee), (Referrer, Udec128), RefereeStats>,
    pub commission: MultiIndex<'a, (Referrer, Referee), (Referrer, Udec128), RefereeStats>,
}
