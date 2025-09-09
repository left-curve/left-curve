#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    super::MAX_PAST_BLOCKS,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity::{self, blocks::latest_block_height},
    itertools::Itertools,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::{
        ops::RangeInclusive,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    },
};

#[derive(Default)]
pub struct MessageSubscription;

impl MessageSubscription {
    async fn get_messages(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
    ) -> Vec<entity::messages::Model> {
        entity::messages::Entity::find()
            .order_by_asc(entity::messages::Column::BlockHeight)
            .order_by_asc(entity::messages::Column::OrderIdx)
            .filter(entity::messages::Column::BlockHeight.is_in(block_heights))
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!(%_e, "`get_messages` error");
            })
            .unwrap_or_default()
    }
}

#[Subscription]
impl MessageSubscription {
    async fn messages<'a>(
        &self,
        ctx: &Context<'a>,
        // This is used to get the older messages in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::messages::Model>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        let received_block_height = Arc::new(AtomicU64::new(latest_block_height as u64));

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "messages",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { Self::get_messages(app_ctx, block_range).await }
        })
        .chain(stream.then(move |block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let received_height = received_block_height.clone();

            async move {
                let current_received = received_height.load(Ordering::Acquire);

                if block_height < current_received {
                    return vec![];
                }

                let messages = Self::get_messages(
                    app_ctx,
                    (current_received + 1) as i64..=block_height as i64,
                )
                .await;

                received_height.store(block_height, Ordering::Release);

                messages
            }
        }))
        .filter_map(|messages| async move {
            if messages.is_empty() {
                None
            } else {
                Some(messages)
            }
        }))
    }
}
