use {
    crate::graphql::types,
    async_graphql::{Object, Result},
    indexer_httpd::context::Context,
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[derive(Default, Debug)]
pub struct TransferQuery {}

#[Object]
impl TransferQuery {
    /// Get a transfer
    async fn transfer(
        &self,
        ctx: &async_graphql::Context<'_>,
        // The block height of the transfer
        height: Option<u64>,
        // The from address of the transfer
        from_address: Option<String>,
        // The to address of the transfer
        to_address: Option<String>,
    ) -> Result<Option<types::transfer::Transfer>> {
        // let app_ctx = ctx.data::<Context>()?;

        // Ok(entity::transfers::Entity::find()
        //    .filter(entity::transfers::Column::BlockHeight.eq(height as i64))
        //    .one(&app_ctx.db)
        //    .await?
        //    .map(|transfer| transfer.into()))

        todo!()
    }
}
