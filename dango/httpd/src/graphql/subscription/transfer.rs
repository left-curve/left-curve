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
        from_address: Option<String>,
        to_address: Option<String>,
    ) -> Vec<Transfer> {
        let mut filter = entity::transfers::Column::BlockHeight.eq(block_height);

        if let Some(from_address) = from_address {
            filter = filter.and(entity::transfers::Column::FromAddress.eq(from_address));
        }

        if let Some(to_address) = to_address {
            filter = filter.and(entity::transfers::Column::ToAddress.eq(to_address));
        }

        entity::transfers::Entity::find()
            .filter(filter)
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
        from_address: Option<String>,
        // The to address of the transfer
        to_address: Option<String>,
    ) -> Result<impl Stream<Item = Vec<Transfer>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let f = from_address.clone();
        let t = to_address.clone();

        Ok(
            once(async move { Self::get_transfers(app_ctx, latest_block_height, f, t).await })
                .chain(
                app_ctx
                    .pubsub
                    .subscribe_block_minted()
                    .await?
                    .then(move |block_height| {
                        let f = from_address.clone();
                        let t = to_address.clone();
                        async move { Self::get_transfers(app_ctx, block_height as i64, f, t).await }
                    }),
            ),
        )
    }
}
