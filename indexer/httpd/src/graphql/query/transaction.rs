use {
    crate::context::Context,
    async_graphql::{connection::*, *},
    indexer_sql::entity::{self, OrderByBlocks, prelude::Transactions},
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "TransactionSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
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

pub type TransactionCursorType = OpaqueCursor<TransactionCursor>;

const MAX_TRANSACTIONS: u64 = 100;

#[derive(Default, Debug)]
pub struct TransactionQuery {}

#[Object]
impl TransactionQuery {
    /// Get transactions
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
        Connection<TransactionCursorType, entity::transactions::Model, EmptyFields, EmptyFields>,
    > {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<TransactionCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::transactions::Entity::find();
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
                        limit = cmp::min(first as u64, MAX_TRANSACTIONS);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_TRANSACTIONS);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_TRANSACTIONS);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_TRANSACTIONS);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_TRANSACTIONS;
                        query = query.limit(MAX_TRANSACTIONS + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                if let Some(block_height) = block_height {
                    query = query
                        .filter(entity::transactions::Column::BlockHeight.eq(block_height as i64));
                }

                if let Some(hash) = hash {
                    query = query.filter(entity::transactions::Column::Hash.eq(&hash));
                }

                if let Some(sender_address) = sender_address {
                    query = query.filter(entity::transactions::Column::Sender.eq(&sender_address));
                }

                let mut transactions = query.all(&app_ctx.db).await?;

                if transactions.len() > limit as usize {
                    transactions.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    transactions.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection
                    .edges
                    .extend(transactions.into_iter().map(|transaction| {
                        Edge::with_additional_fields(
                            OpaqueCursor(transaction.clone().into()),
                            transaction,
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
    query: Select<Transactions>,
    sort_by: SortBy,
    after: &TransactionCursor,
) -> Select<Transactions> {
    match sort_by {
        SortBy::BlockHeightDesc => query.filter(
            Condition::any()
                .add(entity::transactions::Column::BlockHeight.lt(after.block_height))
                .add(
                    Condition::all()
                        .add(entity::transactions::Column::BlockHeight.lte(after.block_height))
                        .add(
                            entity::transactions::Column::TransactionIdx.lt(after.transaction_idx),
                        ),
                ),
        ),
        SortBy::BlockHeightAsc => query.filter(
            Condition::any()
                .add(entity::transactions::Column::BlockHeight.gt(after.block_height))
                .add(
                    Condition::all()
                        .add(entity::transactions::Column::BlockHeight.gte(after.block_height))
                        .add(
                            entity::transactions::Column::TransactionIdx.gt(after.transaction_idx),
                        ),
                ),
        ),
    }
}
