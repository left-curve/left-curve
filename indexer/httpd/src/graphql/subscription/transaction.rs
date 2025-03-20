use {
    crate::graphql::types::transaction::Transaction,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct TransactionSubscription;

#[Subscription]
impl TransactionSubscription {
    async fn transactions<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<Transaction>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let last_block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?;

        let last_block_transactions: Vec<Transaction> = match last_block {
            Some(block) => entity::transactions::Entity::find()
                .order_by_desc(entity::transactions::Column::TransactionIdx)
                .filter(entity::transactions::Column::BlockHeight.eq(block.block_height))
                .all(&app_ctx.db)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
            None => Vec::new(),
        };

        Ok(once(async { last_block_transactions }).chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| async move {
                    entity::transactions::Entity::find()
                        .order_by_desc(entity::transactions::Column::TransactionIdx)
                        .filter(entity::transactions::Column::BlockHeight.eq(block_height as i64))
                        .all(&app_ctx.db)
                        .await
                        .ok()
                        .map(|transactions| transactions.into_iter().map(Into::into).collect())
                        .unwrap_or_default()
                }),
        ))
    }
}
