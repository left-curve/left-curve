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
pub struct PerpsEventSubscription;

impl PerpsEventSubscription {
    /// Get all perps events for the given `block_heights` range.
    async fn get_perps_events(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
        user_addr: Option<String>,
        event_type: Option<String>,
        pair_id: Option<String>,
    ) -> Option<Vec<entity::perps_events::Model>> {
        let mut filter = entity::perps_events::Column::BlockHeight.is_in(block_heights);

        if let Some(user_addr) = user_addr {
            filter = filter.and(entity::perps_events::Column::UserAddr.eq(&user_addr));
        }

        if let Some(event_type) = event_type {
            filter = filter.and(entity::perps_events::Column::EventType.eq(&event_type));
        }

        if let Some(pair_id) = pair_id {
            filter = filter.and(entity::perps_events::Column::PairId.eq(&pair_id));
        }

        let events = entity::perps_events::Entity::find()
            .filter(filter)
            .order_by_asc(entity::perps_events::Column::BlockHeight)
            .order_by_asc(entity::perps_events::Column::Idx)
            .all(&app_ctx.db)
            .await
            .inspect_err(|e| tracing::error!(%e, "`get_perps_events` error"))
            .unwrap_or_default();

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }
}

#[Subscription]
impl PerpsEventSubscription {
    async fn perps_events<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Filter by user address")] user_addr: Option<String>,
        #[graphql(desc = "Filter by event type")] event_type: Option<String>,
        #[graphql(desc = "Filter by trading pair ID")] pair_id: Option<String>,
        #[graphql(desc = "Block height to start from (for reconnection)")]
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<entity::perps_events::Model>> + 'a>
    where
        Self: Sync,
    {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = latest_block_height(&app_ctx.db).await?.unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height.saturating_sub(2).max(1)..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("since_block_height is too old"));
        }

        let user_addr_init = user_addr.clone();
        let event_type_init = event_type.clone();
        let pair_id_init = pair_id.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "perps_events",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Self::get_perps_events(
                    app_ctx,
                    block_range,
                    user_addr_init,
                    event_type_init,
                    pair_id_init,
                )
                .await
            }
        })
        .chain(stream.then(move |block_height| {
            let user_addr = user_addr.clone();
            let event_type = event_type.clone();
            let pair_id = pair_id.clone();

            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Self::get_perps_events(
                    app_ctx,
                    block_height as i64..=block_height as i64,
                    user_addr,
                    event_type,
                    pair_id,
                )
                .await
            }
        }))
        .filter_map(|events| async move { events }))
    }
}
