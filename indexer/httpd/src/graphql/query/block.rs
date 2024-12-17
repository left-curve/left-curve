use {
    crate::{context::Context, graphql::types},
    async_graphql::{Object, Result},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[derive(Default, Debug)]
pub struct BlockQuery {}

#[Object]
impl BlockQuery {
    /// Get a block
    async fn block<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        height: u64,
    ) -> Result<Option<types::block::Block>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(entity::blocks::Entity::find()
            .filter(entity::blocks::Column::BlockHeight.eq(height as i64))
            .one(&app_ctx.db)
            .await?
            .map(|block| block.into()))
    }
}
