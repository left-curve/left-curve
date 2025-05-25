use {
    crate::{
        context::Context,
        graphql::types::{self, event::Event},
    },
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::{self, prelude::Events},
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "EventSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventCursor {
    block_height: u64,
    event_idx: u32,
}

impl From<types::event::Event> for EventCursor {
    fn from(event: types::event::Event) -> Self {
        Self {
            block_height: event.block_height,
            event_idx: event.event_idx,
        }
    }
}

pub type EventCursorType = OpaqueCursor<EventCursor>;

const MAX_EVENTS: u64 = 100;

#[derive(Default, Debug)]
pub struct EventQuery {}

#[Object]
impl EventQuery {
    /// Get events
    async fn events(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<Connection<EventCursorType, Event, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<EventCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::events::Entity::find();
                let sort_by = sort_by.unwrap_or_default();
                let limit;
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_EVENTS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_EVENTS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
                }

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query = query
                            .order_by(entity::events::Column::BlockHeight, Order::Asc)
                            .order_by(entity::events::Column::EventIdx, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query
                            .order_by(entity::events::Column::BlockHeight, Order::Desc)
                            .order_by(entity::events::Column::EventIdx, Order::Desc)
                    },
                }

                let mut events: Vec<types::event::Event> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|event| event.into())
                    .collect::<Vec<_>>();

                if has_before {
                    events.reverse();
                }

                let mut has_more = false;
                if events.len() > limit as usize {
                    events.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
                connection.edges.extend(events.into_iter().map(|event| {
                    Edge::with_additional_fields(
                        OpaqueCursor(event.clone().into()),
                        event,
                        EmptyFields,
                    )
                }));
                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}

fn apply_filter(query: Select<Events>, sort_by: SortBy, after: &EventCursor) -> Select<Events> {
    match sort_by {
        SortBy::BlockHeightAsc => query.filter(
            Condition::any()
                .add(entity::events::Column::BlockHeight.lt(after.block_height))
                .add(
                    entity::events::Column::BlockHeight
                        .eq(after.block_height)
                        .and(entity::events::Column::EventIdx.lt(after.event_idx)),
                ),
        ),
        SortBy::BlockHeightDesc => query.filter(
            Condition::any()
                .add(entity::events::Column::BlockHeight.gt(after.block_height))
                .add(
                    entity::events::Column::BlockHeight
                        .eq(after.block_height)
                        .and(entity::events::Column::EventIdx.gt(after.event_idx)),
                ),
        ),
    }
}
