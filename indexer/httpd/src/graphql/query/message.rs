use {
    crate::{
        context::Context,
        graphql::query::pagination::{CursorFilter, SortByEnum, paginate_models},
    },
    async_graphql::{connection::*, *},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, Condition, QueryFilter, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "MessageSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

impl From<SortBy> for SortByEnum {
    fn from(sort_by: SortBy) -> Self {
        match sort_by {
            SortBy::BlockHeightAsc => SortByEnum::BlockHeightAsc,
            SortBy::BlockHeightDesc => SortByEnum::BlockHeightDesc,
        }
    }
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

#[derive(Default, Debug)]
pub struct MessageQuery {}

#[Object]
impl MessageQuery {
    /// Get paginated messages
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
    ) -> Result<
        Connection<OpaqueCursor<MessageCursor>, entity::messages::Model, EmptyFields, EmptyFields>,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<MessageCursor, entity::messages::Entity, SortBy>(
            app_ctx,
            after,
            before,
            first,
            last,
            sort_by,
            100,
            |query| {
                let mut query = query;

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

                query
            },
        )
        .await
    }
}

impl CursorFilter<MessageCursor> for Select<entity::messages::Entity> {
    fn apply_cursor_filter(self, sort_by: SortByEnum, cursor: &MessageCursor) -> Self {
        match sort_by {
            SortByEnum::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::messages::Column::BlockHeight.gt(cursor.block_height))
                    .add(
                        entity::messages::Column::BlockHeight
                            .gte(cursor.block_height)
                            .and(entity::messages::Column::OrderIdx.gt(cursor.order_idx)),
                    ),
            ),
            SortByEnum::BlockHeightDesc => self.filter(
                Condition::any()
                    .add(entity::messages::Column::BlockHeight.lt(cursor.block_height))
                    .add(
                        entity::messages::Column::BlockHeight
                            .lte(cursor.block_height)
                            .and(entity::messages::Column::OrderIdx.lt(cursor.order_idx)),
                    ),
            ),
        }
    }
}
