use {async_graphql::*, candle::CandleSubscription};

pub mod candle;

#[derive(MergedSubscription, Default)]
pub struct Subscription(CandleSubscription);
