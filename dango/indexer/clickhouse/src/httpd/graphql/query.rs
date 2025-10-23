use {async_graphql::MergedObject, candle::CandleQuery, trade::TradeQuery};

pub mod candle;
pub mod trade;

#[derive(MergedObject, Default)]
pub struct Query(CandleQuery, TradeQuery);
