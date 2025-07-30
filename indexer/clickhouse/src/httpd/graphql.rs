#[cfg(feature = "metrics")]
use indexer_httpd::graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics};

use {
    crate::{
        context::Context,
        entities::{candle_query::MAX_ITEMS, pair_price::PairPrice},
    },
    async_graphql::{
        EmptyMutation, EmptySubscription, Schema,
        extensions::{self as AsyncGraphqlExtensions},
    },
    futures::StreamExt,
    indexer_httpd::graphql::telemetry::SentryExtension,
    std::time::Duration,
    tokio::time::sleep,
};

pub mod query;
pub mod subscription;

pub(crate) type AppSchema = Schema<query::Query, EmptyMutation, EmptySubscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    #[cfg(feature = "metrics")]
    init_graphql_metrics();

    #[allow(unused_mut)]
    let mut schema_builder = Schema::build(
        query::Query::default(),
        EmptyMutation,
        EmptySubscription,
    )
    .extension(AsyncGraphqlExtensions::Logger)
    // .extension(AsyncGraphqlExtensions::Tracing)
    .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    schema_builder
        .data(app_ctx)
        .limit_complexity(300)
        .limit_depth(20)
        .finish()
}

/// Must be called to ensure the candle cache is updated
pub async fn update_candle_cache(app_ctx: Context) {
    loop {
        if let Ok(mut subscription) = app_ctx.pubsub.subscribe_block_minted().await {
            while let Some(block_height) = subscription.next().await {
                // TODO: get pairs from dex contract
                let pairs = PairPrice::all_pairs(app_ctx.clickhouse_client())
                    .await
                    .unwrap_or_default();

                let mut candle_cache = app_ctx.candle_cache.write().await;
                if let Err(_err) = candle_cache
                    .update_pairs(app_ctx.clickhouse_client(), &pairs, block_height)
                    .await
                {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_err,"Failed to update candle cache");
                    continue;
                }

                candle_cache.compact_keep_n(MAX_ITEMS);

                if let Err(_err) = app_ctx
                    .candle_pubsub
                    .publish_candles_cached(block_height)
                    .await
                {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_err, "Failed to publish candles cached");
                }
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!("Sleeping before next candle cache update");

        sleep(Duration::from_millis(500)).await;
    }
}
