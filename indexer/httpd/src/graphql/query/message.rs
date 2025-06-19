use {
    crate::context::Context,
    async_graphql::{connection::*, *},
    indexer_sql::entity::{self, OrderByBlocks, prelude::Messages},
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "MessageSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageCursor {
    block_height: u64,
    order_idx: u32,
}

impl From<entity::messages::Model> for MessageCursor {
    fn from(message: entity::messages::Model) -> Self {
        Self {
            block_height: message.block_height as u64,
            order_idx: message.order_idx as u32,
        }
    }
}

pub type MessageCursorType = OpaqueCursor<MessageCursor>;

const MAX_MESSAGES: u64 = 100;

#[derive(Default, Debug)]
pub struct MessageQuery {}

#[Object]
impl MessageQuery {
    /// Get messages
    async fn messages(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
        block_height: Option<u64>,
        method_name: Option<String>,
        contract_addr: Option<String>,
        sender_addr: Option<String>,
    ) -> Result<Connection<MessageCursorType, entity::messages::Model, EmptyFields, EmptyFields>>
    {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<MessageCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::messages::Entity::find();
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
                        limit = cmp::min(first as u64, MAX_MESSAGES);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_MESSAGES);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_MESSAGES);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_MESSAGES);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_MESSAGES;
                        query = query.limit(MAX_MESSAGES + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                if let Some(block_height) = block_height {
                    query =
                        query.filter(entity::messages::Column::BlockHeight.eq(block_height as i64));
                }

                if let Some(method_name) = method_name {
                    query = query.filter(entity::messages::Column::MethodName.eq(method_name));
                }

                if let Some(contract_addr) = contract_addr {
                    query = query.filter(entity::messages::Column::ContractAddr.eq(contract_addr));
                }

                if let Some(sender_addr) = sender_addr {
                    query = query.filter(entity::messages::Column::SenderAddr.eq(sender_addr));
                }

                let mut messages = query.all(&app_ctx.db).await?;

                if messages.len() > limit as usize {
                    messages.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    messages.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.edges.extend(messages.into_iter().map(|message| {
                    Edge::with_additional_fields(
                        OpaqueCursor(message.clone().into()),
                        message,
                        EmptyFields,
                    )
                }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}

fn apply_filter(
    query: Select<Messages>,
    sort_by: SortBy,
    after: &MessageCursor,
) -> Select<Messages> {
    match sort_by {
        SortBy::BlockHeightDesc => query.filter(
            Condition::any()
                .add(entity::messages::Column::BlockHeight.lt(after.block_height))
                .add(
                    entity::messages::Column::BlockHeight
                        .lte(after.block_height)
                        .and(entity::messages::Column::OrderIdx.lt(after.order_idx)),
                ),
        ),
        SortBy::BlockHeightAsc => query.filter(
            Condition::any()
                .add(entity::messages::Column::BlockHeight.gt(after.block_height))
                .add(
                    entity::messages::Column::BlockHeight
                        .gte(after.block_height)
                        .and(entity::messages::Column::OrderIdx.gt(after.order_idx)),
                ),
        ),
    }
}
