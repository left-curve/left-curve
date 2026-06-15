use {
    async_graphql::*, perps_candle::PerpsCandleSubscription,
    perps_pair_stats::PerpsPairStatsSubscription,
};

pub mod perps_candle;
pub mod perps_pair_stats;

#[derive(MergedSubscription, Default)]
pub struct ClickhouseSubscription(PerpsCandleSubscription, PerpsPairStatsSubscription);
