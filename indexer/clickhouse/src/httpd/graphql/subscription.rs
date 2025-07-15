use {async_graphql::*, candle::CandleSubscription};

pub mod candle;

pub const MAX_PAST_CANDLES: usize = 100;

#[derive(MergedSubscription, Default)]
pub struct Subscription(CandleSubscription);
