#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;

use {
    crate::{
        entities::{CandleInterval, candle::Candle},
        indexer::candles::cache,
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
pub struct CandleSubscription;

#[Subscription]
impl CandleSubscription {
    /// Get candles for a given base and quote denom, interval
    async fn candles<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        base_denom: String,
        quote_denom: String,
        interval: CandleInterval,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        later_than: Option<DateTime<Utc>>,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        limit: Option<usize>,
    ) -> Result<impl Stream<Item = Vec<Candle>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let candle_cache = app_ctx.candle_cache.clone();
        let cache_key =
            cache::CandleCacheKey::new(base_denom.clone(), quote_denom.clone(), interval);

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "candles",
            "subscription",
        ));

        let received_block_height = Arc::new(AtomicU64::new(0));

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Ok(candle_cache
                    .read()
                    .await
                    .get_last_candle(&cache_key)
                    .cloned())
            }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |current_block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let cache_key =
                cache::CandleCacheKey::new(base_denom.clone(), quote_denom.clone(), interval);
            let candle_cache = app_ctx.candle_cache.clone();

            let previous_block_height = received_block_height.fetch_max(current_block_height, Ordering::Release);

            async move {
                if current_block_height < previous_block_height {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        previous_block_height,
                        current_block_height,
                        "Skip candle, pubsub block_height is lower than previous"
                    );
                    return Ok(None);
                }

                let last_candle = candle_cache
                    .read()
                    .await
                    .get_last_candle(&cache_key)
                    .cloned();

                let Some(candle) = last_candle else {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        previous_block_height,
                        current_block_height,
                        "No current candle, expected if chain started"
                    );
                    return Ok(None);
                };

                if candle.max_block_height < current_block_height {
                    let candle_cache = candle_cache
                        .read()
                        .await;
                    let candle_cache_block_heights = candle_cache
                        .candles
                        .get(&cache_key)
                        .map(|candles| candles.iter().map(|c| c.max_block_height).collect::<Vec<_>>())
                        .unwrap_or_default();
                    let pair_prices_block_heights = candle_cache.pair_prices.keys().cloned().collect::<Vec<_>>();

                    let _pair_price_current_block_exists = candle_cache.pair_price_for_block(current_block_height).is_some();

                    drop(candle_cache);

                    let _cache_max_block_height = candle_cache_block_heights.iter().max();
                    let _cache_min_block_height = candle_cache_block_heights.iter().min();
                    let _cache_len = candle_cache_block_heights.len();

                    let _pair_prices_min_block_height = pair_prices_block_heights.iter().min();
                    let _pair_prices_max_block_height = pair_prices_block_heights.iter().max();

                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        previous_block_height,
                        current_block_height,
                        %candle.max_block_height,
                        cache_max_block_height=?_cache_max_block_height,
                        cache_min_block_height=?_cache_min_block_height,
                        cache_len=?_cache_len,
                        pair_prices_min_block_height=?_pair_prices_min_block_height,
                        pair_prices_max_block_height=?_pair_prices_max_block_height,
                        pair_price_current_block_exists=_pair_price_current_block_exists,
                        interval = ?interval,
                        "Skip candle, it has older block_height than received pubsub block_height"
                    );

                    return Ok(None);
                }

                Ok(Some(candle))
            }
        }))
        .filter_map(|candle: Result<Option<Candle>>| async move {
            match candle {
                Ok(Some(candle)) => {
                    Some(vec![candle])
                },
                Ok(None) => None,
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error getting candles: {_err:?}");

                    None
                },
            }
        }))
    }
}
