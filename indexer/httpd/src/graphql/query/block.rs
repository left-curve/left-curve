use crate::context::Context;
use crate::graphql::types;
use async_graphql::{Object, Result};
use indexer_sql::entity;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;

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
