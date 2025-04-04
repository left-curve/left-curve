use {
    crate::graphql::types::message::Message,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity::{self, blocks::latest_block_height},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct MessageSubscription;

impl MessageSubscription {
    async fn get_messages(app_ctx: &crate::context::Context, block_height: i64) -> Vec<Message> {
        entity::messages::Entity::find()
            .order_by_asc(entity::messages::Column::OrderIdx)
            .filter(entity::messages::Column::BlockHeight.eq(block_height))
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!("get_messages error: {_e:?}");
            })
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[Subscription]
impl MessageSubscription {
    async fn messages<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<Message>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        Ok(
            once(async move { Self::get_messages(app_ctx, latest_block_height).await }).chain(
                app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_messages(app_ctx, block_height as i64).await
                    },
                ),
            ),
        )
    }
}
