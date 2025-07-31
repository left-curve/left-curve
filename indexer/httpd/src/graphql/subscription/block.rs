use {
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct BlockSubscription;

#[Subscription]
impl BlockSubscription {
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

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { last_block }
        })
        .chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| {
                    #[cfg(feature = "metrics")]
                    let _guard = gauge_guard.clone();

                    async move {
                        entity::blocks::Entity::find()
                            .filter(entity::blocks::Column::BlockHeight.eq(block_height as i64))
                            .one(&app_ctx.db)
                            .await
                            .inspect_err(|_e| {
                                #[cfg(feature = "tracing")]
                                tracing::error!(%_e, "Block error");
                            })
                            .ok()
                            .unwrap_or_default()
                    }
                }),
        )
        .filter_map(|block| async { block }))
    }
}
