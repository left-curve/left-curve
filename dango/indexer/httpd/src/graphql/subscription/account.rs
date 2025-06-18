use {
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity,
    futures_util::stream::{StreamExt, once},
    indexer_httpd::graphql::subscription::MAX_PAST_BLOCKS,
    indexer_sql::entity::blocks::latest_block_height,
    itertools::Itertools,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::ops::RangeInclusive,
};

#[derive(Default)]
pub struct AccountSubscription;

impl AccountSubscription {
    async fn get_accounts(
        app_ctx: &indexer_httpd::context::Context,
        block_heights: RangeInclusive<i64>,
        username: Option<String>,
    ) -> Vec<entity::accounts::Model> {
        let mut filter = entity::accounts::Column::CreatedBlockHeight.is_in(block_heights);

        if let Some(username) = username {
            filter = filter.and(entity::users::Column::Username.eq(&username));
        }

        let query = entity::accounts::Entity::find()
            .order_by_desc(entity::accounts::Column::CreatedBlockHeight)
            .find_with_related(entity::users::Entity)
            .filter(filter);

        query
            .all(&app_ctx.db)
            .await
            .inspect_err(|e| tracing::error!(%e, "`get_accounts` error"))
            .unwrap_or_default()
            .into_iter()
            .map(|(account, _)| account)
            .collect::<Vec<_>>()
    }
}

#[Subscription]
impl AccountSubscription {
    async fn accounts<'a>(
        &self,
        ctx: &Context<'a>,
        username: Option<String>,
        // The block height of the transfer
        // This is used to get the older account creations in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::accounts::Model>> + 'a> {
        let app_ctx = ctx.data::<indexer_httpd::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("since_block_height is too old"));
        }

        let u = username.clone();

        Ok(
            once(async { Self::get_accounts(app_ctx, block_range, u).await })
                .chain(
                    app_ctx
                        .pubsub
                        .subscribe_block_minted()
                        .await?
                        .then(move |block_height| {
                            let u = username.clone();
                            async move {
                                Self::get_accounts(
                                    app_ctx,
                                    block_height as i64..=block_height as i64,
                                    u,
                                )
                                .await
                            }
                        }),
                )
                .filter_map(|maybe_accounts| async move {
                    if maybe_accounts.is_empty() {
                        None
                    } else {
                        Some(maybe_accounts)
                    }
                }),
        )
    }
}
