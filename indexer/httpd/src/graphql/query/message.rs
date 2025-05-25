use {
    crate::{
        context::Context,
        graphql::types::{self, message::Message},
    },
    async_graphql::{connection::*, *},
    indexer_sql::entity::{self, prelude::Messages},
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
    },
    serde::{Deserialize, Serialize},
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

impl From<types::message::Message> for MessageCursor {
    fn from(message: types::message::Message) -> Self {
        Self {
            block_height: message.block_height,
            order_idx: message.order_idx,
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
        block_height: Option<u64>,
        method_name: Option<String>,
        contract_addr: Option<String>,
        sender_addr: Option<String>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<Connection<MessageCursorType, Message, EmptyFields, EmptyFields>> {
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
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_MESSAGES);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_MESSAGES);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
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

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query = query
                            .order_by(entity::messages::Column::BlockHeight, Order::Asc)
                            .order_by(entity::messages::Column::OrderIdx, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query
                            .order_by(entity::messages::Column::BlockHeight, Order::Desc)
                            .order_by(entity::messages::Column::OrderIdx, Order::Desc)
                    },
                }

                let mut messages: Vec<types::message::Message> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|message| message.into())
                    .collect::<Vec<_>>();

                if has_before {
                    messages.reverse();
                }

                let mut has_more = false;
                if messages.len() > limit as usize {
                    messages.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
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
        SortBy::BlockHeightAsc => query.filter(
            Condition::any()
                .add(entity::messages::Column::BlockHeight.lt(after.block_height))
                .add(
                    entity::messages::Column::BlockHeight
                        .eq(after.block_height)
                        .and(entity::messages::Column::OrderIdx.lt(after.order_idx)),
                ),
        ),
        SortBy::BlockHeightDesc => query.filter(
            Condition::any()
                .add(entity::messages::Column::BlockHeight.gt(after.block_height))
                .add(
                    entity::messages::Column::BlockHeight
                        .eq(after.block_height)
                        .and(entity::messages::Column::OrderIdx.gt(after.order_idx)),
                ),
        ),
    }
}
