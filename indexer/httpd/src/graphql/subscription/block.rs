#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
pub struct BlockSubscription;

#[Subscription]
impl BlockSubscription {
    // Block are guaranteed to be streamed in order
    async fn block<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = entity::blocks::Model> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let last_block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "block",
            "subscription",
        ));

        let received_block_height = Arc::new(AtomicU64::new(
            last_block.as_ref().map_or(0, |block| block.block_height) as u64,
        ));

        #[cfg(feature = "tracing")]
        tracing::debug!(?received_block_height, "Subscribing to block events");

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { last_block.map(|block| vec![block]).unwrap_or_default() }
        })
        .chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .filter_map(move |block_height| {
                    #[cfg(feature = "metrics")]
                    let _guard = gauge_guard.clone();

                    let received_height = received_block_height.clone();

                    async move {
                        let current_received = received_height.load(Ordering::Acquire);
                        if block_height < current_received {
                            #[cfg(feature = "tracing")]
                            tracing::debug!(current_received, block_height, "Skip block");
                            return None;
                        }

                        #[cfg(feature = "tracing")]
                        tracing::debug!(current_received, block_height, "Streaming blocks");

                        let blocks = entity::blocks::Entity::find()
                            .filter(
                                entity::blocks::Column::BlockHeight
                                    .gt(current_received as i64)
                                    .and(
                                        entity::blocks::Column::BlockHeight
                                            .lte(block_height as i64),
                                    ),
                            )
                            .order_by_asc(entity::blocks::Column::BlockHeight)
                            .all(&app_ctx.db)
                            .await
                            .inspect_err(|_e| {
                                #[cfg(feature = "tracing")]
                                tracing::error!(%_e, "Block error");
                            })
                            .unwrap_or_default();

                        received_height.store(block_height, Ordering::Release);

                        Some(blocks)
                    }
                }),
        )
        .flat_map(futures_util::stream::iter))
    }
}
