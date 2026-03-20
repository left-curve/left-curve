use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity,
    indexer_httpd::{
        context::Context,
        graphql::query::pagination::{CursorFilter, CursorOrder, Reversible, paginate_models},
    },
    sea_orm::{ColumnTrait, Condition, Order, QueryFilter, QueryOrder, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "PerpsEventSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

impl Reversible for SortBy {
    fn rev(&self) -> Self {
        match self {
            SortBy::BlockHeightAsc => SortBy::BlockHeightDesc,
            SortBy::BlockHeightDesc => SortBy::BlockHeightAsc,
        }
    }
}

impl From<SortBy> for Order {
    fn from(sort_by: SortBy) -> Self {
        match sort_by {
            SortBy::BlockHeightAsc => Order::Asc,
            SortBy::BlockHeightDesc => Order::Desc,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerpsEventCursor {
    block_height: u64,
    idx: i32,
}

impl From<entity::perps_events::Model> for PerpsEventCursor {
    fn from(event: entity::perps_events::Model) -> Self {
        Self {
            block_height: event.block_height as u64,
            idx: event.idx,
        }
    }
}

#[derive(Default, Debug)]
pub struct PerpsEventQuery {}

#[Object]
impl PerpsEventQuery {
    /// Get paginated perps events
    async fn perps_events(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
        #[graphql(desc = "Filter by user address")] user_addr: Option<String>,
        #[graphql(desc = "Filter by event type")] event_type: Option<String>,
        #[graphql(desc = "Filter by trading pair ID")] pair_id: Option<String>,
        #[graphql(desc = "Filter by block height")] block_height: Option<u64>,
    ) -> Result<
        Connection<
            OpaqueCursor<PerpsEventCursor>,
            entity::perps_events::Model,
            EmptyFields,
            EmptyFields,
        >,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models(
            app_ctx,
            after,
            before,
            first,
            last,
            sort_by,
            100,
            |query, _| {
                Box::pin(async move {
                    let mut query = query;

                    if let Some(user_addr) = user_addr {
                        query = query.filter(entity::perps_events::Column::UserAddr.eq(&user_addr));
                    }

                    if let Some(event_type) = event_type {
                        query =
                            query.filter(entity::perps_events::Column::EventType.eq(&event_type));
                    }

                    if let Some(pair_id) = pair_id {
                        query = query.filter(entity::perps_events::Column::PairId.eq(&pair_id));
                    }

                    if let Some(block_height) = block_height {
                        query = query.filter(
                            entity::perps_events::Column::BlockHeight.eq(block_height as i64),
                        );
                    }

                    Ok(query)
                })
            },
        )
        .await
    }
}

impl CursorFilter<SortBy, PerpsEventCursor> for Select<entity::perps_events::Entity> {
    fn cursor_filter(self, sort: &SortBy, cursor: &PerpsEventCursor) -> Self {
        match sort {
            SortBy::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::perps_events::Column::BlockHeight.gt(cursor.block_height as i64))
                    .add(
                        entity::perps_events::Column::BlockHeight
                            .gte(cursor.block_height as i64)
                            .and(entity::perps_events::Column::Idx.gt(cursor.idx)),
                    ),
            ),
            SortBy::BlockHeightDesc => self.filter(
                Condition::any()
                    .add(entity::perps_events::Column::BlockHeight.lt(cursor.block_height as i64))
                    .add(
                        entity::perps_events::Column::BlockHeight
                            .lte(cursor.block_height as i64)
                            .and(entity::perps_events::Column::Idx.lt(cursor.idx)),
                    ),
            ),
        }
    }
}

impl CursorOrder<SortBy> for Select<entity::perps_events::Entity> {
    fn cursor_order(self, sort: SortBy) -> Self {
        let order: Order = sort.into();

        self.order_by(entity::perps_events::Column::BlockHeight, order.clone())
            .order_by(entity::perps_events::Column::Idx, order)
    }
}
