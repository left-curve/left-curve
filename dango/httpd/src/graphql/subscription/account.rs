use {
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity,
    futures_util::stream::{StreamExt, once},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[derive(Default)]
pub struct AccountSubscription;

#[Subscription]
impl AccountSubscription {
    async fn accounts<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<impl Stream<Item = Vec<entity::accounts::Model>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let last_accounts = entity::accounts::Entity::find()
            .order_by_desc(entity::accounts::Column::CreatedBlockHeight)
            .all(&app_ctx.db)
            .await?;

        Ok(once(async { Some(last_accounts) })
            .chain(
                app_ctx
                    .pubsub
                    .subscribe_block_minted()
                    .await?
                    .then(move |block_height| {
                        let db = app_ctx.db.clone();
                        async move {
                            entity::accounts::Entity::find()
                                .filter(
                                    entity::accounts::Column::CreatedBlockHeight.eq(block_height),
                                )
                                .all(&db)
                                .await
                                .ok()
                        }
                    }),
            )
            .filter_map(|maybe_accounts| async move { maybe_accounts }))
    }
}
