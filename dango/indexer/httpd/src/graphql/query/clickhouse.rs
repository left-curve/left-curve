use {
    super::{
        candle::CandleQuery, pair_stats::PairStatsQuery, perps_candle::PerpsCandleQuery,
        perps_fees::PerpsFeesQuery, perps_pair_stats::PerpsPairStatsQuery, trade::TradeQuery,
    },
    async_graphql::MergedObject,
};

#[derive(MergedObject, Default)]
pub struct ClickhouseQuery(
    CandleQuery,
    TradeQuery,
    PairStatsQuery,
    PerpsCandleQuery,
    PerpsPairStatsQuery,
    PerpsFeesQuery,
);
