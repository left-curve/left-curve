use {
    crate::{
        context::Context,
        entities::{
            CandleInterval, perps_candle::PerpsCandle, perps_candle_query::PerpsCandleQueryBuilder,
        },
        indexer::perps_candles::cache,
    },
    async_graphql::{types::connection::*, *},
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerpsCandleCursor {
    time_start: DateTime<Utc>,
}

impl From<PerpsCandle> for PerpsCandleCursor {
    fn from(candle: PerpsCandle) -> Self {
        Self {
            time_start: candle.time_start,
        }
    }
}

#[derive(Default, Debug)]
pub struct PerpsCandleQuery;

#[Object]
impl PerpsCandleQuery {
    /// Get paginated perps candles
    async fn perps_candles(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Pair ID (e.g. perp/btcusd)")] pair_id: String,
        #[graphql(desc = "Interval")] interval: CandleInterval,
        earlier_than: Option<DateTime<Utc>>,
        later_than: Option<DateTime<Utc>>,
    ) -> Result<Connection<OpaqueCursor<PerpsCandleCursor>, PerpsCandle, EmptyFields, EmptyFields>>
    {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        query_with::<OpaqueCursor<PerpsCandleCursor>, _, _, _, _>(
            after,
            None,
            first,
            None,
            |after, _, first, _| async move {
                // Can use cache
                if after.is_none() && first.is_none() {
                    let candle_cache = app_ctx.perps_candle_cache.read().await;

                    let cache_key = cache::PerpsCandleCacheKey::new(pair_id.clone(), interval);

                    if candle_cache.date_interval_available(&cache_key, earlier_than, later_than)
                        && let Some(cached_candles) = candle_cache.get_candles(&cache_key)
                    {
                        let mut connection = Connection::new(false, true);

                        connection
                            .edges
                            .extend(cached_candles.iter().rev().map(|candle| {
                                Edge::with_additional_fields(
                                    OpaqueCursor(candle.clone().into()),
                                    candle.clone(),
                                    EmptyFields,
                                )
                            }));

                        return Ok::<_, async_graphql::Error>(connection);
                    }
                }

                let mut query_builder = PerpsCandleQueryBuilder::new(interval, pair_id.clone());

                if let Some(earlier_than) = earlier_than {
                    query_builder = query_builder.with_earlier_than(earlier_than);
                }

                if let Some(later_than) = later_than {
                    query_builder = query_builder.with_later_than(later_than);
                }

                if let Some(first) = first {
                    query_builder = query_builder.with_limit(first);
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
