use {
    crate::graphql::types::block::Block,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct BlockSubscription;

#[Subscription]
impl BlockSubscription {
    async fn block<'a>(&self, ctx: &Context<'a>) -> Result<impl Stream<Item = Block> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let last_block: Option<Block> = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?
            .map(Into::into);

        Ok(once(async { last_block })
            .chain(app_ctx.pubsub.subscribe_block_minted().await?.then(
                move |block_height| async move {
                    entity::blocks::Entity::find()
                        .filter(entity::blocks::Column::BlockHeight.eq(block_height as i64))
                        .one(&app_ctx.db)
                        .await
                        .inspect_err(|_e| {
                            #[cfg(feature = "tracing")]
                            tracing::error!("block error: {_e:?}");
                        })
                        .ok()
                        .unwrap_or_default()
                        .map(Into::into)
                },
            ))
            .filter_map(|block| async { block }))
    }
}
