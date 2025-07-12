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

        // 1m interval query:
        //         SELECT
        //       quote_denom,
        //       base_denom,
        //       CAST(high AS String) AS high,
        //       CAST(low AS String) AS low,
        //       CAST(open AS String) AS open,
        //       CAST(close AS String) AS close,
        //       CAST(volume AS String) AS volume,
        //       toUnixTimestamp64Milli(time_start) AS time_start,
        //       toUnixTimestamp64Milli(time_start) + 59999 AS time_end  -- For 1m (60000 ms - 1)
        //   FROM pair_prices_1m
        //   WHERE quote_denom = {quote:String}
        //   AND base_denom = {base:String}
        //   AND time_start >= fromUnixTimestamp64({startTime:UInt64} / 1000, 3)
        //   AND time_start < fromUnixTimestamp64({endTime:UInt64} / 1000, 3)
        //   ORDER BY time_start ASC

        // general query
        // SELECT
        // {quote:String} AS quote_denom,
        // {base:String} AS base_denom,
        // CAST(MIN(clearing_price) AS String) AS low,
        // CAST(MAX(clearing_price) AS String) AS high,
        // CAST(argMin(clearing_price, created_at) AS String) AS open,
        // CAST(argMax(clearing_price, created_at) AS String) AS close,
        // CAST(SUM(volume) AS String) AS volume,
        // toStartOfInterval(created_at, INTERVAL {interval:UInt32} SECOND) AS time_start_internal,
        // toUnixTimestamp64Milli(time_start_internal) AS time_start,
        // toUnixTimestamp64Milli(time_start_internal) + ({interval:UInt32} * 1000) - 1 AS time_end
        // FROM pair_prices
        // WHERE quote_denom = {quote:String}
        // AND base_denom = {base:String}
        // AND created_at >= fromUnixTimestamp64({startTime:UInt64} / 1000, 3)
        // AND created_at < fromUnixTimestamp64({endTime:UInt64} / 1000, 3)
        // GROUP BY time_start_internal
        // ORDER BY time_start_internal ASC

        todo!()
    }
}
