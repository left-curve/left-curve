use {
    crate::graphql::types::event::Event,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct EventSubscription;

impl EventSubscription {
    async fn get_events(app_ctx: &crate::context::Context, block_height: i64) -> Vec<Event> {
        entity::events::Entity::find()
            .order_by_asc(entity::events::Column::EventIdx)
            .filter(entity::events::Column::BlockHeight.eq(block_height))
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!("get_events error: {_e:?}");
            })
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[Subscription]
impl EventSubscription {
    async fn events<'a>(&self, ctx: &Context<'a>) -> Result<impl Stream<Item = Vec<Event>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?
            .map(|block| block.block_height)
            .unwrap_or_default();

        Ok(
            once(async move { Self::get_events(app_ctx, latest_block_height).await }).chain(
                app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_events(app_ctx, block_height as i64).await
                    },
                ),
            ),
        )
    }
}
