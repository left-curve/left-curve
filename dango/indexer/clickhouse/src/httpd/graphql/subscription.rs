use {
    async_graphql::*, candle::CandleSubscription, perps_candle::PerpsCandleSubscription,
    trade::TradeSubscription,
};

pub mod candle;
pub mod perps_candle;
pub mod trade;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    CandleSubscription,
    TradeSubscription,
    PerpsCandleSubscription,
);
