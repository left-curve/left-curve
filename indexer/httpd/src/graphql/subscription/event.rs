use {
    super::MAX_PAST_BLOCKS,
    crate::graphql::types::event::Event,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    indexer_sql::entity,
    itertools::Itertools,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::ops::RangeInclusive,
};

#[derive(Default)]
pub struct EventSubscription;

impl EventSubscription {
    async fn get_events(
        app_ctx: &crate::context::Context,
        block_heights: RangeInclusive<i64>,
    ) -> Vec<Event> {
        entity::events::Entity::find()
            .order_by_asc(entity::events::Column::BlockHeight)
            .order_by_asc(entity::events::Column::EventIdx)
            .filter(entity::events::Column::BlockHeight.is_in(block_heights))
            .all(&app_ctx.db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!("get_events error: {_e:?}");
            })
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[Subscription]
impl EventSubscription {
    async fn events<'a>(
        &self,
        ctx: &Context<'a>,
        // This is used to get the older events in case of disconnection
        since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = Vec<Event>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let latest_block_height = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&app_ctx.db)
            .await?
            .map(|block| block.block_height)
            .unwrap_or_default();

        let block_range = match since_block_height {
            Some(block_height) => block_height as i64..=latest_block_height,
            None => latest_block_height..=latest_block_height,
        };

        if block_range.try_len().unwrap_or(0) > MAX_PAST_BLOCKS {
            return Err(async_graphql::Error::new("since_block_height is too old"));
        }

        Ok(
            once(async move { Self::get_events(app_ctx, block_range).await })
                .chain(app_ctx.pubsub.subscribe_block_minted().await?.then(
                    move |block_height| async move {
                        Self::get_events(app_ctx, block_height as i64..=block_height as i64).await
                    },
                ))
                .filter_map(|events| async move {
                    if events.is_empty() {
                        None
                    } else {
                        Some(events)
                    }
                }),
        )
    }
}
