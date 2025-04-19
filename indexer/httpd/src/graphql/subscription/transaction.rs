use {
    crate::graphql::types::transaction::Transaction,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity::{self, blocks::latest_block_height},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct TransactionSubscription;

impl TransactionSubscription {
    async fn get_transactions(
        app_ctx: &crate::context::Context,
        block_height: i64,
    ) -> Vec<Transaction> {
        entity::transactions::Entity::find()
            .order_by_asc(entity::transactions::Column::TransactionIdx)
            .filter(entity::transactions::Column::BlockHeight.eq(block_height))
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!("get_transactions error: {_e:?}");
            })
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[Subscription]
impl TransactionSubscription {
    async fn transactions<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<Transaction>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        Ok(
            once(async move { Self::get_transactions(app_ctx, latest_block_height).await })
                .chain(app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_transactions(app_ctx, block_height as i64).await
                    },
                ))
                .filter_map(|transactions| async move {
                    if transactions.is_empty() {
                        None
                    } else {
                        Some(transactions)
                    }
                }),
        )
    }
}
