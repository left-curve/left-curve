use {
    crate::graphql::types::event::Event,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct EventSubscription;

#[Subscription]
impl EventSubscription {
    async fn events<'a>(&self, ctx: &Context<'a>) -> Result<impl Stream<Item = Vec<Event>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let last_block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?;

        let last_block_events: Vec<Event> = match last_block {
            Some(block) => entity::events::Entity::find()
                .order_by_desc(entity::events::Column::EventIdx)
                .filter(entity::events::Column::BlockHeight.eq(block.block_height))
                .all(&app_ctx.db)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
            None => Vec::new(),
        };

        Ok(once(async { last_block_events }).chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| async move {
                    entity::events::Entity::find()
                        .order_by_desc(entity::events::Column::EventIdx)
                        .filter(entity::events::Column::BlockHeight.eq(block_height as i64))
                        .all(&app_ctx.db)
                        .await
                        .ok()
                        .map(|events| events.into_iter().map(Into::into).collect())
                        .unwrap_or_default()
                }),
        ))
    }
}
