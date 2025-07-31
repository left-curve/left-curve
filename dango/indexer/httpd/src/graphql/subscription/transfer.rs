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
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct TransferSubscription;

impl TransferSubscription {
    /// Get all transfers for the given `block_heights` range.
    async fn get_transfers(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
        address: Option<String>,
        username: Option<String>,
    ) -> Option<Vec<entity::transfers::Model>> {
        let mut filter = entity::transfers::Column::BlockHeight.is_in(block_heights);

        if let Some(username) = username {
            if let Ok(accounts) = entity::accounts::Entity::find()
                .find_also_related(entity::users::Entity)
                .filter(entity::users::Column::Username.eq(username))
                .all(&app_ctx.db)
                .await
            {
                let addresses = accounts
                    .into_iter()
                    .map(|(account, _)| account.address)
                    .collect::<Vec<_>>();

                filter = filter.and(
                    entity::transfers::Column::FromAddress
                        .is_in(&addresses)
                        .or(entity::transfers::Column::ToAddress.is_in(&addresses)),
                );
            }
        }

        if let Some(address) = address {
            filter = filter.and(
                entity::transfers::Column::FromAddress
                    .eq(&address)
                    .or(entity::transfers::Column::ToAddress.eq(&address)),
            );
        }

        let transfers = entity::transfers::Entity::find()
            .filter(filter)
            .order_by_asc(entity::transfers::Column::BlockHeight)
            .order_by_asc(entity::transfers::Column::Idx)
            .all(&app_ctx.db)
            .await
            .inspect_err(|e| tracing::error!(%e, "`get_transfers` error"))
            .unwrap_or_default();

        if transfers.is_empty() {
            None
        } else {
            Some(transfers)
        }
    }
}

#[Subscription]
impl TransferSubscription {
    async fn transfers<'a>(
        &self,
        ctx: &Context<'a>,
        address: Option<String>,
        username: Option<String>,
        // The block height of the transfer
        // This is used to get the older transfers in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::transfers::Model>> + 'a>
    where
        Self: Sync,
    {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("since_block_height is too old"));
        }

        let a = address.clone();
        let u = username.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "transfers",
            "subscription",
        ));

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { Self::get_transfers(app_ctx, block_range, a, u).await }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
            let a = address.clone();
            let u = username.clone();

            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Self::get_transfers(app_ctx, block_height as i64..=block_height as i64, a, u).await
            }
        }))
        .filter_map(|transfers| async move { transfers }))
    }
}
