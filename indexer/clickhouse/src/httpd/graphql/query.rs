use {async_graphql::MergedObject, candle::CandleQuery};

pub mod candle;

#[derive(MergedObject, Default)]
pub struct Query(CandleQuery);
