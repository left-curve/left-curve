#![allow(unused_variables)]

use {
    crate::{
        context::Context,
        entities::candle::{Candle, CandleInterval},
    },
    async_graphql::{types::connection::*, *},
    chrono::{DateTime, Utc},
    grug::Timestamp,
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

const MAX_ITEMS: u64 = 100;

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
                let mut limit = MAX_ITEMS;
                let mut has_previous_page = false;

                let interval_str = interval.to_string();

                let mut query = format!(
                    "SELECT quote_denom, base_denom, time_start, open, high, low, close, volume_base, volume_quote, '{interval_str}' as interval
                    FROM {table_name}
                    WHERE quote_denom = ? AND base_denom = ?",
                );

                let mut params: Vec<String> = vec![quote_denom.clone(), base_denom.clone()];

                if let Some(earlier_than_start_time) = earlier_than   {
                    query.push_str(" AND time_start <= ?");
                    params.push(Timestamp::from(earlier_than_start_time.naive_utc()).to_rfc3339_string());


                }

                if let Some(later_than_start_time) = later_than {
                    query.push_str(" AND time_start >= ?");
                    params.push(Timestamp::from(later_than_start_time.naive_utc()).to_rfc3339_string());
                }

                if let Some(after) = after {
                    query.push_str(" AND time_start < ?");
                    params.push(Timestamp::from(after.time_start.naive_utc()).to_rfc3339_string());
                    has_previous_page = true;
                }

                if let Some(first) = first {
                    limit = std::cmp::min(first as u64, MAX_ITEMS);
                }

                query.push_str(" ORDER BY time_start DESC");
                query.push_str(&format!(" LIMIT {}", limit + 1));

                let mut cursor_query = clickhouse_client.query(&query);
                for param in params {
                    cursor_query = cursor_query.bind(param);
                }

                let mut rows: Vec<Candle> = cursor_query.fetch_all().await?;

                let has_next_page = rows.len() > limit as usize - 1;
                if has_next_page {
                    rows.pop();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);

                connection.edges.extend(rows.into_iter().map(|candle| {
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
