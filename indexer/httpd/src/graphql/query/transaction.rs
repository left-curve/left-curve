use {
    crate::{
        context::Context,
        graphql::query::pagination::{CursorFilter, CursorOrder, Reversible, paginate_models},
    },
    async_graphql::{connection::*, *},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, Condition, Order, QueryFilter, QueryOrder, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "TransactionSortBy")]
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
pub struct TransactionCursor {
    block_height: u64,
    transaction_idx: u32,
}

impl From<entity::transactions::Model> for TransactionCursor {
    fn from(transaction: entity::transactions::Model) -> Self {
        Self {
            block_height: transaction.block_height as u64,
            transaction_idx: transaction.transaction_idx as u32,
        }
    }
}

#[derive(Default, Debug)]
pub struct TransactionQuery {}

#[Object]
impl TransactionQuery {
    /// Get paginated transactions
    async fn transactions(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
        hash: Option<String>,
        block_height: Option<u64>,
        sender_address: Option<String>,
    ) -> Result<
        Connection<
            OpaqueCursor<TransactionCursor>,
            entity::transactions::Model,
            EmptyFields,
            EmptyFields,
        >,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<TransactionCursor, entity::transactions::Entity, SortBy>(
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

                    if let Some(block_height) = block_height {
                        query = query.filter(
                            entity::transactions::Column::BlockHeight.eq(block_height as i64),
                        );
                    }

                    if let Some(hash) = hash {
                        query = query.filter(entity::transactions::Column::Hash.eq(&hash));
                    }

                    if let Some(sender_address) = sender_address {
                        query =
                            query.filter(entity::transactions::Column::Sender.eq(&sender_address));
                    }

                    Ok(query)
                })
            },
        )
        .await
    }
}

impl CursorFilter<SortBy, TransactionCursor> for Select<entity::transactions::Entity> {
    fn cursor_filter(self, sort: &SortBy, cursor: &TransactionCursor) -> Self {
        match sort {
            SortBy::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::transactions::Column::BlockHeight.gt(cursor.block_height))
                    .add(
                        Condition::all()
                            .add(entity::transactions::Column::BlockHeight.gte(cursor.block_height))
                            .add(
                                entity::transactions::Column::TransactionIdx
                                    .gt(cursor.transaction_idx),
                            ),
                    ),
            ),
            SortBy::BlockHeightDesc => self.filter(
                Condition::any()
                    .add(entity::transactions::Column::BlockHeight.lt(cursor.block_height))
                    .add(
                        Condition::all()
                            .add(entity::transactions::Column::BlockHeight.lte(cursor.block_height))
                            .add(
                                entity::transactions::Column::TransactionIdx
                                    .lt(cursor.transaction_idx),
                            ),
                    ),
            ),
        }
    }
}

impl CursorOrder<SortBy> for Select<entity::transactions::Entity> {
    fn cursor_order(self, sort: SortBy) -> Self {
        let order: Order = sort.into();
        self.order_by(entity::transactions::Column::BlockHeight, order.clone())
            .order_by(entity::transactions::Column::TransactionIdx, order)
    }
}
