use {
    super::{
        candle::CandleSubscription, pair_stats::PairStatsSubscription,
        perps_candle::PerpsCandleSubscription, perps_pair_stats::PerpsPairStatsSubscription,
        trade::TradeSubscription,
    },
    async_graphql::*,
};

#[derive(MergedSubscription, Default)]
pub struct ClickhouseSubscription(
    CandleSubscription,
    TradeSubscription,
    PerpsCandleSubscription,
    PairStatsSubscription,
    PerpsPairStatsSubscription,
);
