use {
    crate::graphql::types,
    async_graphql::{Object, Result},
    dango_indexer_sql::entity,
    indexer_httpd::context::Context,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[derive(Default, Debug)]
pub struct TransferQuery {}

#[Object]
impl TransferQuery {
    /// Get a transfer
    async fn transfers(
        &self,
        ctx: &async_graphql::Context<'_>,
        // The block height of the transfer
        height: Option<u64>,
        // The from address of the transfer
        from_address: Option<String>,
        // The to address of the transfer
        to_address: Option<String>,
    ) -> Result<Vec<types::transfer::Transfer>> {
        let app_ctx = ctx.data::<Context>()?;

        let mut query = entity::transfers::Entity::find();

        if let Some(height) = height {
            query = query.filter(entity::transfers::Column::BlockHeight.eq(height as i64));
        }

        if let Some(from_address) = from_address {
            query = query.filter(entity::transfers::Column::FromAddress.eq(from_address));
        }

        if let Some(to_address) = to_address {
            query = query.filter(entity::transfers::Column::ToAddress.eq(to_address));
        }

        Ok(query
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(|transfer| transfer.into())
            .collect::<Vec<_>>())
    }
}
