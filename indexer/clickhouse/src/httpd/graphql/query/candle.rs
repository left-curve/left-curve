use {
    crate::{
        cache,
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

        query_with::<OpaqueCursor<CandleCursor>, _, _, _, _>(
            after,
            None,
            first,
            None,
            |after, _, first, _| async move {
                // Can use cache
                if after.is_none() && first.is_none() {
                    let candle_cache = app_ctx.candle_cache.read().await;

                    let cache_key = cache::CandleCacheKey::new(
                        base_denom.clone(),
                        quote_denom.clone(),
                        interval,
                    );

                    if candle_cache.date_interval_available(&cache_key, earlier_than, later_than) {
                        if let Some(cached_candles) = candle_cache.get_candles(&cache_key) {
                            let mut connection = Connection::new(false, true);

                            connection.edges.extend(cached_candles.iter().map(|candle| {
                                Edge::with_additional_fields(
                                    OpaqueCursor(candle.clone().into()),
                                    candle.clone(),
                                    EmptyFields,
                                )
                            }));

                            return Ok::<_, async_graphql::Error>(connection);
                        }
                    }
                }

                let mut query_builder =
                    CandleQueryBuilder::new(interval, base_denom.clone(), quote_denom.clone());

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
