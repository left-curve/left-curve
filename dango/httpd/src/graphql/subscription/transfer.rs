use {
    crate::graphql::types::transfer::Transfer,
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity,
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity::blocks::latest_block_height,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct TransferSubscription;

impl TransferSubscription {
    /// Get all transfers for the given block_height
    async fn get_transfers(
        app_ctx: &indexer_httpd::context::Context,
        block_height: i64,
    ) -> Vec<Transfer> {
        entity::transfers::Entity::find()
            .filter(entity::transfers::Column::BlockHeight.eq(block_height))
            .order_by_asc(entity::transfers::Column::Idx)
            .all(&app_ctx.db)
            .await
            .inspect_err(|e| tracing::error!("get_transfers error: {:?}", e))
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[Subscription]
impl TransferSubscription {
    async fn transfers<'a>(
        &self,
        ctx: &Context<'a>,
        // The from address of the transfer
        _from_address: Option<String>,
        // The to address of the transfer
        _to_address: Option<String>,
    ) -> Result<impl Stream<Item = Vec<Transfer>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        Ok(
            once(async move { Self::get_transfers(app_ctx, latest_block_height).await }).chain(
                app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_transfers(app_ctx, block_height as i64).await
                    },
                ),
            ),
        )
    }
}
