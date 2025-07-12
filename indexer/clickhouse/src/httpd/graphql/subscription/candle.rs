#![allow(unused_variables)]
#![allow(unused_imports)]

use {
    crate::{entities::pair_price::PairPrice, httpd::graphql::subscription::MAX_PAST_BLOCKS},
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    itertools::Itertools,
    std::ops::RangeInclusive,
};

#[derive(Default)]
pub struct CandleSubscription;

impl CandleSubscription {
    async fn get_candles(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
    ) -> Vec<PairPrice> {
        todo!()
    }
}

#[Subscription]
impl CandleSubscription {
    async fn candle<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        pair_id: Option<String>,
        // This is used to get the older candles in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<PairPrice>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = 0; // latest_block_height(&app_ctx.clickhouse_client).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        Ok(
            once(async move { Self::get_candles(app_ctx, block_range).await })
                .chain(app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_candles(app_ctx, block_height as i64..=block_height as i64).await
                    },
                ))
                .filter_map(|pair_prices| async move {
                    if pair_prices.is_empty() {
                        None
                    } else {
                        Some(pair_prices)
                    }
                }),
        )
    }
}
