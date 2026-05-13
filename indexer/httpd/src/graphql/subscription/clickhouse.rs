use {
    async_graphql::*, candle::CandleSubscription, pair_stats::PairStatsSubscription,
    perps_candle::PerpsCandleSubscription, perps_pair_stats::PerpsPairStatsSubscription,
    trade::TradeSubscription,
};

pub mod candle;
pub mod pair_stats;
pub mod perps_candle;
pub mod perps_pair_stats;
pub mod trade;

#[derive(MergedSubscription, Default)]
pub struct ClickhouseSubscription(
    CandleSubscription,
    TradeSubscription,
    PerpsCandleSubscription,
    PairStatsSubscription,
    PerpsPairStatsSubscription,
);
