use {
    crate::{
        context::Context,
        graphql::query::pagination::{CursorFilter, CursorOrder, Reversible, paginate_models},
    },
    async_graphql::{types::connection::*, *},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, Condition, Order, QueryFilter, QueryOrder, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "EventSortBy")]
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
pub struct EventCursor {
    block_height: u64,
    event_idx: u32,
}

impl From<entity::events::Model> for EventCursor {
    fn from(event: entity::events::Model) -> Self {
        Self {
            block_height: event.block_height as u64,
            event_idx: event.event_idx as u32,
        }
    }
}

#[derive(Default, Debug)]
pub struct EventQuery {}

#[Object]
impl EventQuery {
    /// Get paginated events
    async fn events(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<
        Connection<OpaqueCursor<EventCursor>, entity::events::Model, EmptyFields, EmptyFields>,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<EventCursor, entity::events::Entity, SortBy>(
            app_ctx,
            after,
            before,
            first,
            last,
            sort_by,
            100,
            |query, _| Box::pin(async move { Ok(query) }),
        )
        .await
    }
}

impl CursorFilter<SortBy, EventCursor> for Select<entity::events::Entity> {
    fn cursor_filter(self, sort: &SortBy, cursor: &EventCursor) -> Self {
        match sort {
            SortBy::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::events::Column::BlockHeight.gt(cursor.block_height))
                    .add(
                        entity::events::Column::BlockHeight
                            .gte(cursor.block_height)
                            .and(entity::events::Column::EventIdx.gt(cursor.event_idx)),
                    ),
            ),
            SortBy::BlockHeightDesc => self.filter(
                Condition::any()
                    .add(entity::events::Column::BlockHeight.lt(cursor.block_height))
                    .add(
                        entity::events::Column::BlockHeight
                            .lte(cursor.block_height)
                            .and(entity::events::Column::EventIdx.lt(cursor.event_idx)),
                    ),
            ),
        }
    }
}

impl CursorOrder<SortBy> for Select<entity::events::Entity> {
    fn cursor_order(self, sort: SortBy) -> Self {
        let order: Order = sort.into();

        self.order_by(entity::events::Column::BlockHeight, order.clone())
            .order_by(entity::events::Column::EventIdx, order)
    }
}
