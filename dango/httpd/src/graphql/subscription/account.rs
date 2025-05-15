use {
    crate::graphql::types::account::Account,
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
    ) -> Result<impl Stream<Item = Vec<Account>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let last_accounts: Vec<Account> = entity::accounts::Entity::find()
            .order_by_desc(entity::accounts::Column::CreatedBlockHeight)
            .all(&app_ctx.db)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

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
                                // .order_by_asc(entity::accounts::Column::Username)
                                .all(&db)
                                .await
                                .ok()
                                .map(|accounts| {
                                    accounts
                                        .into_iter()
                                        .map(Into::into)
                                        .collect::<Vec<Account>>()
                                })
                        }
                    }),
            )
            .filter_map(|maybe_accounts| async move { maybe_accounts }))
    }
}
