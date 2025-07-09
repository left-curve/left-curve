#![allow(unused_variables)]

use {
    crate::{context::Context, entities::pair_price::PairPrice},
    async_graphql::{types::connection::*, *},
    chrono::NaiveDateTime,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandleCursor {
    block_height: u64,
}

impl From<PairPrice> for CandleCursor {
    fn from(pair_price: PairPrice) -> Self {
        Self {
            block_height: pair_price.block_height,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "CandleInterval")]
pub enum CandleInterval {
    #[default]
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
}

#[derive(Default, Debug)]
pub struct CandleQuery;

#[Object]
impl CandleQuery {
    async fn candles(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        pair_id: String,
        interval: CandleInterval,
        start_time: Option<NaiveDateTime>,
        end_time: Option<NaiveDateTime>,
    ) -> Result<Connection<OpaqueCursor<CandleCursor>, PairPrice, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        todo!()
    }
}
