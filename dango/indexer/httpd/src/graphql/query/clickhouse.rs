use {
    async_graphql::MergedObject, perps_candle::PerpsCandleQuery, perps_fees::PerpsFeesQuery,
    perps_pair_stats::PerpsPairStatsQuery,
};

pub mod perps_candle;
pub mod perps_fees;
pub mod perps_pair_stats;

#[derive(MergedObject, Default)]
pub struct ClickhouseQuery(PerpsCandleQuery, PerpsPairStatsQuery, PerpsFeesQuery);
