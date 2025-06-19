use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::{self, OrderByBlocks, prelude::Events},
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
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

impl From<entity::events::Model> for EventCursor {
    fn from(event: entity::events::Model) -> Self {
        Self {
            block_height: event.block_height as u64,
            event_idx: event.event_idx as u32,
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
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<Connection<EventCursorType, entity::events::Model, EmptyFields, EmptyFields>> {
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

                let mut has_next_page = false;
                let mut has_previous_page = false;

                match (last, sort_by) {
                    (None, SortBy::BlockHeightAsc) | (Some(_), SortBy::BlockHeightDesc) => {
                        query = query.order_by_blocks_asc()
                    },
                    (None, SortBy::BlockHeightDesc) | (Some(_), SortBy::BlockHeightAsc) => {
                        query = query.order_by_blocks_desc()
                    },
                }

                match (first, after, last, before) {
                    (Some(first), None, None, None) => {
                        limit = cmp::min(first as u64, MAX_EVENTS);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_EVENTS);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_EVENTS);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_EVENTS);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_EVENTS;
                        query = query.limit(MAX_EVENTS + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }


                let mut events = query.all(&app_ctx.db).await?;

                if events.len() > limit as usize {
                    events.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    events.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
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
        SortBy::BlockHeightDesc => query.filter(
            Condition::any()
                .add(entity::events::Column::BlockHeight.lt(after.block_height))
                .add(
                    entity::events::Column::BlockHeight
                        .lte(after.block_height)
                        .and(entity::events::Column::EventIdx.lt(after.event_idx)),
                ),
        ),
        SortBy::BlockHeightAsc => query.filter(
            Condition::any()
                .add(entity::events::Column::BlockHeight.gt(after.block_height))
                .add(
                    entity::events::Column::BlockHeight
                        .gte(after.block_height)
                        .and(entity::events::Column::EventIdx.gt(after.event_idx)),
                ),
        ),
    }
}
