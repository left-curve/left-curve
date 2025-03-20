use {
    crate::graphql::types::transfer::Transfer,
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity,
    futures_util::stream::{StreamExt, once},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct TransferSubscription;

#[Subscription]
impl TransferSubscription {
    async fn transfers<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<Transfer>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let last_transfers: Vec<Transfer> = entity::transfers::Entity::find()
            .order_by_desc(entity::transfers::Column::BlockHeight)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(once(async { Some(last_transfers) })
            .chain(
                app_ctx
                    .pubsub
                    .subscribe_block_minted()
                    .await?
                    .then(move |block_height| {
                        let db = app_ctx.db.clone();
                        async move {
                            entity::transfers::Entity::find()
                                .filter(
                                    entity::transfers::Column::BlockHeight.eq(block_height as i64),
                                )
                                .order_by_asc(entity::transfers::Column::Idx)
                                .all(&db)
                                .await
                                .ok()
                                .map(|transfers| {
                                    transfers
                                        .into_iter()
                                        .map(Into::into)
                                        .collect::<Vec<Transfer>>()
                                })
                        }
                    }),
            )
            .filter_map(|maybe_transfers| async move { maybe_transfers }))
    }
}
