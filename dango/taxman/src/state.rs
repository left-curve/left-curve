use {
    dango_types::{
        account_factory::UserIndex,
        taxman::{Config, Referee, RefereeData, Referrer, ShareRatio, UserReferralData},
    },
    grug::{IndexedMap, Item, Map, MultiIndex, Timestamp, Udec128, Uint128},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_FEE: Item<(Config, Uint128)> = Item::new("withheld_fee");

// Given a referee, find his referrer.
pub const REFEREE_TO_REFERRER: Map<Referee, Referrer> = Map::new("referee_to_referrer");

// How much of the fee the referrer wishes to share with the referee.
pub const FEE_SHARE_RATIO: Map<Referrer, ShareRatio> = Map::new("fee_share_ratio");

// Stores the total (cumulative) data for an user.
pub const USER_CUMULATIVE_DATA: Map<(UserIndex, Timestamp), UserReferralData> =
    Map::new("user_cumulative_data");

// Stores the statistics of referees for each referrer.
pub const REFERRER_TO_REFEREE_STATISTICS: IndexedMap<
    (Referrer, Referee),
    RefereeData,
    ReferrerStatisticsIndex,
> = IndexedMap::new("referrer_statistics", ReferrerStatisticsIndex {
    register_at: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.registered_at),
        "referrer_statistics",
        "referrer_statistics__register_at",
    ),
    volume: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.volume),
        "referrer_statistics",
        "referrer_statistics__volume",
    ),
    commission: MultiIndex::new(
        |(referrer, _), data| (*referrer, data.commission_rebounded),
        "referrer_statistics",
        "referrer_statistics__commission",
    ),
});

#[grug::index_list((Referrer, Referee), RefereeData)]
pub struct ReferrerStatisticsIndex<'a> {
    pub register_at: MultiIndex<'a, (Referrer, Referee), (Referrer, Timestamp), RefereeData>,
    pub volume: MultiIndex<'a, (Referrer, Referee), (Referrer, Udec128), RefereeData>,
    pub commission: MultiIndex<'a, (Referrer, Referee), (Referrer, Udec128), RefereeData>,
}

// mod test {
//     use grug::MockStorage;

//     use crate::REFERRER_TO_REFEREE_STATISTICS;

//     #[test]
//     fn test() {
//         let mut storage = MockStorage::new();

//         REFERRER_TO_REFEREE_STATISTICS.idx.volume.sub_prefix(1);
//         REFERRER_TO_REFEREE_STATISTICS.idx.commission.sub_prefix(1);
//     }
// }
