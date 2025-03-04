use {
    crate::graphql::types::message::Message,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{once, StreamExt},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct MessageSubscription;

#[Subscription]
impl MessageSubscription {
    async fn messages<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<Message>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let last_block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?;

        let last_block_messages: Vec<Message> = match last_block {
            Some(block) => entity::messages::Entity::find()
                .order_by_desc(entity::messages::Column::OrderIdx)
                .filter(entity::messages::Column::BlockHeight.eq(block.block_height))
                .all(&app_ctx.db)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
            None => Vec::new(),
        };

        Ok(once(async { last_block_messages }).chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| async move {
                    entity::messages::Entity::find()
                        .order_by_desc(entity::messages::Column::OrderIdx)
                        .filter(entity::messages::Column::BlockHeight.eq(block_height as i64))
                        .all(&app_ctx.db)
                        .await
                        .ok()
                        .map(|messages| messages.into_iter().map(Into::into).collect())
                        .unwrap_or_default()
                }),
        ))
    }
}
