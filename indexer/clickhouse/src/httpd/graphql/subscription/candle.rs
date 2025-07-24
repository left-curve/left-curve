use {
    crate::entities::{
        CandleInterval, candle::Candle, candle_query::CandleQueryBuilder,
        pair_price_query::PairPriceQueryBuilder,
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
};

#[derive(Default)]
pub struct CandleSubscription;

impl CandleSubscription {
    /// Get candles for a given base and quote denom, interval, and later than time.
    async fn get_candles(
        app_ctx: &crate::context::Context,
        base_denom: String,
        quote_denom: String,
        interval: CandleInterval,
        later_than: Option<DateTime<Utc>>,
        limit: Option<usize>,
        block_height: Option<u64>,
    ) -> Result<Vec<Candle>> {
        // We will be called for every block minted, so we need to check if there
        // is a new pair price later than the given block height. If there is no new
        // pair price, we don't want to return any candles.
        if let Some(block_height) = block_height {
            let pair_price_query_builder =
                PairPriceQueryBuilder::new(base_denom.clone(), quote_denom.clone())
                    .with_later_block_height(block_height);

            let pair_price = pair_price_query_builder
                .fetch_one(app_ctx.clickhouse_client())
                .await?;

            if pair_price.is_none() {
                return Ok(vec![]);
            }
        }

        let mut query_builder =
            CandleQueryBuilder::new(interval, base_denom.clone(), quote_denom.clone());

        if let Some(later_than) = later_than {
            query_builder = query_builder.with_later_than(later_than);
        }

        if let Some(limit) = limit {
            query_builder = query_builder.with_limit(limit);
        }

        let result = query_builder.fetch_all(app_ctx.clickhouse_client()).await?;

        Ok(result.candles)
    }
}

#[Subscription]
impl CandleSubscription {
    /// Get candles for a given base and quote denom, interval, and later than time.
    /// If `limit` is provided, it will be used to limit the number of candles returned.
    /// If `limit` is not provided, it will default to MAX_PAST_CANDLES.
    /// If `limit` is greater than MAX_PAST_CANDLES, it will be set to MAX_PAST_CANDLES.
    /// If `later_than` is provided, it will be used to filter the candles returned.
    async fn candles<'a>(
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
                limit,
                None,
            )
            .await
        })
        .chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| {
                    let base_denom = base_denom.clone();
                    let quote_denom = quote_denom.clone();
                    let interval = interval;
                    let later_than = later_than;

                    async move {
                        Self::get_candles(
                            app_ctx,
                            base_denom,
                            quote_denom,
                            interval,
                            later_than,
                            limit,
                            Some(block_height),
                        )
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
                    tracing::error!("Error getting candles: {_err:?}");

                    None
                },
            }
        }))
    }
}
