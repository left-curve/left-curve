use {
    async_graphql::MergedObject, candle::CandleQuery, pair_stats::PairStatsQuery, trade::TradeQuery,
};

pub mod candle;
pub mod pair_stats;
pub mod trade;

#[derive(MergedObject, Default)]
pub struct Query(CandleQuery, TradeQuery, PairStatsQuery);
