use {
    async_graphql::MergedObject, candle::CandleQuery, pair_stats::PairStatsQuery,
    perps_candle::PerpsCandleQuery, perps_pair_stats::PerpsPairStatsQuery, trade::TradeQuery,
};

pub mod candle;
pub mod pair_stats;
pub mod perps_candle;
pub mod perps_pair_stats;
pub mod trade;

#[derive(MergedObject, Default)]
pub struct Query(
    CandleQuery,
    TradeQuery,
    PairStatsQuery,
    PerpsCandleQuery,
    PerpsPairStatsQuery,
);
