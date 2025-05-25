use {
    crate::{
        context::Context,
        graphql::types::{self, transaction::Transaction},
    },
    async_graphql::{connection::*, *},
    indexer_sql::entity::{self, prelude::Transactions},
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
    },
    serde::{Deserialize, Serialize},
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

impl From<types::transaction::Transaction> for TransactionCursor {
    fn from(transaction: types::transaction::Transaction) -> Self {
        Self {
            block_height: transaction.block_height,
            transaction_idx: transaction.transaction_idx,
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
        hash: Option<String>,
        block_height: Option<u64>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
        sender_address: Option<String>,
    ) -> Result<Connection<TransactionCursorType, Transaction, EmptyFields, EmptyFields>> {
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
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_TRANSACTIONS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_TRANSACTIONS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
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

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query = query
                            .order_by(entity::transactions::Column::BlockHeight, Order::Asc)
                            .order_by(entity::transactions::Column::TransactionIdx, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query
                            .order_by(entity::transactions::Column::BlockHeight, Order::Desc)
                            .order_by(entity::transactions::Column::TransactionIdx, Order::Desc)
                    },
                }

                let mut transactions: Vec<types::transaction::Transaction> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|transaction| transaction.into())
                    .collect::<Vec<_>>();

                if has_before {
                    transactions.reverse();
                }

                let mut has_more = false;
                if transactions.len() > limit as usize {
                    transactions.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
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
        SortBy::BlockHeightAsc => query.filter(
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
        SortBy::BlockHeightDesc => query.filter(
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
