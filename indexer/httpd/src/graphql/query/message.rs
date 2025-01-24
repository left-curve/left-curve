use {
    crate::{context::Context, graphql::types},
    async_graphql::{Object, Result},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[derive(Default, Debug)]
pub struct MessageQuery {}

#[Object]
impl MessageQuery {
    /// Get a block
    async fn messages(
        &self,
        ctx: &async_graphql::Context<'_>,
        height: u64,
    ) -> Result<Vec<types::message::Message>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(entity::messages::Entity::find()
            .filter(entity::messages::Column::BlockHeight.eq(height as i64))
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(|message| message.into())
            .collect())
    }
}
