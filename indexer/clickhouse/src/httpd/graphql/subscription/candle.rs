use {
    crate::{
        entities::candle::{Candle, CandleInterval},
        httpd::graphql::subscription::MAX_PAST_CANDLES,
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
    grug::Timestamp,
};

#[derive(Default)]
pub struct CandleSubscription;

impl CandleSubscription {
    async fn get_candles(
        app_ctx: &crate::context::Context,
        base_denom: String,
        quote_denom: String,
        interval: CandleInterval,
        later_than: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<Candle>> {
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

        let interval_str = interval.to_string();

        let mut query = format!(
            "SELECT quote_denom, base_denom, time_start, open, high, low, close, volume_base, volume_quote, '{interval_str}' as interval
            FROM {table_name}
            WHERE quote_denom = ? AND base_denom = ?",
        );

        let mut params: Vec<String> = vec![quote_denom.clone(), base_denom.clone()];

        if let Some(later_than_start_time) = later_than {
            query.push_str(" AND time_start >= ?");
            params.push(Timestamp::from(later_than_start_time.naive_utc()).to_rfc3339_string());
        }

        query.push_str(" ORDER BY time_start DESC");
        query.push_str(&format!(" LIMIT {limit}"));

        Ok(app_ctx
            .clickhouse_client()
            .query(&query)
            .bind(params)
            .fetch_all::<Candle>()
            .await?)
    }
}

#[Subscription]
impl CandleSubscription {
    /// Get candles for a given base and quote denom, interval, and later than time.
    /// If `limit` is provided, it will be used to limit the number of candles returned.
    /// If `limit` is not provided, it will default to MAX_PAST_CANDLES.
    /// If `limit` is greater than MAX_PAST_CANDLES, it will be set to MAX_PAST_CANDLES.
    /// If `later_than` is provided, it will be used to filter the candles returned.
    async fn candle<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        base_denom: String,
        quote_denom: String,
        interval: CandleInterval,
        later_than: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<impl Stream<Item = Vec<Candle>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let base_denom_clone = base_denom.clone();
        let quote_denom_clone = quote_denom.clone();

        Ok(once(async move {
            Self::get_candles(
                app_ctx,
                base_denom_clone,
                quote_denom_clone,
                interval,
                later_than,
                std::cmp::min(limit.unwrap_or(MAX_PAST_CANDLES), MAX_PAST_CANDLES),
            )
            .await
        })
        .chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |_| {
                    let base_denom = base_denom.clone();
                    let quote_denom = quote_denom.clone();
                    let interval = interval;
                    let later_than = later_than;
                    async move {
                        Self::get_candles(app_ctx, base_denom, quote_denom, interval, later_than, 1)
                            .await
                    }
                }),
        )
        .filter_map(|candles| async move {
            match candles {
                Ok(candles) => {
                    if candles.is_empty() {
                        None
                    } else {
                        Some(candles)
                    }
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error getting candles: {:?}", _err);
                    None
                },
            }
        }))
    }
}
