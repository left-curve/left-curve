use {
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity,
    futures_util::stream::{StreamExt, once},
    indexer_httpd::graphql::subscription::MAX_PAST_BLOCKS,
    indexer_sql::entity::blocks::latest_block_height,
    itertools::Itertools,
    sea_orm::{
        ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    },
    std::ops::RangeInclusive,
};
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct AccountSubscription;

impl AccountSubscription {
    async fn get_accounts(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
        username: Option<String>,
    ) -> Vec<entity::accounts::Model> {
        let mut query = entity::accounts::Entity::find()
            .filter(entity::accounts::Column::CreatedBlockHeight.is_in(block_heights));

        if let Some(username) = username {
            query = query
                .join(
                    JoinType::InnerJoin,
                    entity::accounts::Relation::AccountUser.def(),
                )
                .join(
                    JoinType::InnerJoin,
                    entity::accounts_users::Relation::User.def(),
                )
                .filter(entity::users::Column::Username.eq(&username));
        }

        let query = query.order_by_desc(entity::accounts::Column::CreatedBlockHeight);

        query
            .all(&app_ctx.db)
            .await
            .inspect_err(|e| tracing::error!(%e, "`get_accounts` error"))
            .unwrap_or_default()
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
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            // Removing 2 blocks so we have past accounts on the first call.
            None => latest_block_height.saturating_sub(2).max(1)..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("`since_block_height` is too old"));
        }

        let u = username.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "accounts",
            "subscription",
        ));

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { Self::get_accounts(app_ctx, block_range, u).await }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
            let u = username.clone();

            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Self::get_accounts(app_ctx, block_height as i64..=block_height as i64, u).await
            }
        }))
        .filter_map(|maybe_accounts| async move {
            if maybe_accounts.is_empty() {
                None
            } else {
                Some(maybe_accounts)
            }
        }))
    }
}
