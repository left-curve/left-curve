#![allow(unused_variables)]

use {
    crate::{
        context::Context,
        entities::{CandleInterval, candle::Candle, candle_query::CandleQueryBuilder},
    },
    async_graphql::{types::connection::*, *},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandleCursor {
    time_start: DateTime<Utc>,
}

impl From<Candle> for CandleCursor {
    fn from(candle: Candle) -> Self {
        Self {
            time_start: candle.time_start,
        }
    }
}

#[derive(Default, Debug)]
pub struct CandleQuery;

#[Object]
impl CandleQuery {
    /// Get paginated candles
    async fn candles(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Base denom")] base_denom: String,
        #[graphql(desc = "Quote denom")] quote_denom: String,
        #[graphql(desc = "Interval")] interval: CandleInterval,
        earlier_than: Option<DateTime<Utc>>,
        later_than: Option<DateTime<Utc>>,
    ) -> Result<Connection<OpaqueCursor<CandleCursor>, Candle, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let table_name = match interval {
            CandleInterval::OneSecond => "pair_prices_1s",
            CandleInterval::OneMinute => "pair_prices_1m",
            CandleInterval::FiveMinutes => "pair_prices_5m",
            CandleInterval::FifteenMinutes => "pair_prices_15m",
            CandleInterval::OneHour => "pair_prices_1h",
            CandleInterval::FourHours => "pair_prices_4h",
            CandleInterval::OneDay => "pair_prices_1d",
            CandleInterval::OneWeek => "pair_prices_1w",
        };

        query_with::<OpaqueCursor<CandleCursor>, _, _, _, _>(
            after,
            None,
            first,
            None,
            |after, before, first, last| async move {
                let mut query_builder =
                    CandleQueryBuilder::new(interval, base_denom.clone(), quote_denom.clone());

                if let Some(earlier_than) = earlier_than {
                    query_builder = query_builder.with_earlier_than(earlier_than);
                }

                if let Some(later_than) = later_than {
                    query_builder = query_builder.with_later_than(later_than);
                }

                if let Some(after) = after {
                    query_builder = query_builder.with_after(after.time_start);
                }

                let result = query_builder.fetch_all(clickhouse_client).await?;

                let mut connection =
                    Connection::new(result.has_previous_page, result.has_next_page);

                connection
                    .edges
                    .extend(result.candles.into_iter().map(|candle| {
                        Edge::with_additional_fields(
                            OpaqueCursor(candle.clone().into()),
                            candle,
                            EmptyFields,
                        )
                    }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}
