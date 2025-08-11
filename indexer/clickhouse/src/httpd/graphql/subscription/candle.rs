#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;

use {
    crate::{
        cache,
        entities::{CandleInterval, candle::Candle},
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
                    .map(|candle| vec![candle.clone()]))
            }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let cache_key =
                cache::CandleCacheKey::new(base_denom.clone(), quote_denom.clone(), interval);
            let candle_cache = app_ctx.candle_cache.clone();

            let received_height = received_block_height.clone();

            async move {
                let current_received = received_height.load(Ordering::Acquire);
                if block_height < current_received {
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        current_received,
                        block_height,
                        "Skip candle, same block_height, shouldn't happen..."
                    );
                    return Ok(None);
                }

                let candle = candle_cache
                    .read()
                    .await
                    .get_last_candle(&cache_key)
                    .map(|candle| vec![candle.clone()]);

                received_height.store(block_height, Ordering::Release);

                Ok(candle)
            }
        }))
        .filter_map(|candles: Result<Option<Vec<Candle>>>| async move {
            match candles {
                Ok(Some(candles)) => {
                    if candles.is_empty() {
                        None
                    } else {
                        Some(candles)
                    }
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
