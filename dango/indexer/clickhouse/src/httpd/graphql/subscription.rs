use {async_graphql::*, candle::CandleSubscription, trade::TradeSubscription};

pub mod candle;
pub mod trade;

#[derive(MergedSubscription, Default)]
pub struct Subscription(CandleSubscription, TradeSubscription);
