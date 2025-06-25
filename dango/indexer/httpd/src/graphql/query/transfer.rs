use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity,
    indexer_httpd::{
        context::Context,
        graphql::query::pagination::{CursorFilter, CursorOrder, Reversible, paginate_models},
    },
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, JoinType, Order, QueryFilter, QueryOrder, QuerySelect,
        QueryTrait, RelationTrait, Select,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "TransferSortBy")]
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
pub struct TransferCursor {
    block_height: u64,
    idx: i32,
}

impl From<entity::transfers::Model> for TransferCursor {
    fn from(transfer: entity::transfers::Model) -> Self {
        Self {
            block_height: transfer.block_height as u64,
            idx: transfer.idx,
        }
    }
}

#[derive(Default, Debug)]
pub struct TransferQuery {}

#[Object]
impl TransferQuery {
    /// Get paginated transfers
    async fn transfers(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
        // The block height of the transfer
        block_height: Option<u64>,
        // The from address of the transfer
        from_address: Option<String>,
        // The to address of the transfer
        to_address: Option<String>,
        username: Option<String>,
    ) -> Result<
        Connection<
            OpaqueCursor<TransferCursor>,
            entity::transfers::Model,
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

                    if let Some(block_height) = block_height {
                        query = query
                            .filter(entity::transfers::Column::BlockHeight.eq(block_height as i64));
                    }

                    if let Some(from_address) = from_address {
                        query =
                            query.filter(entity::transfers::Column::FromAddress.eq(&from_address));
                    }

                    if let Some(to_address) = to_address {
                        query = query.filter(entity::transfers::Column::ToAddress.eq(&to_address));
                    }

                    if let Some(username) = username {
                        // NOTE: keeping the "safe" version for now, until I confirm in production the subquery code works correctly.

                        // let accounts = entity::accounts::Entity::find()
                        //     .find_also_related(entity::users::Entity)
                        //     .filter(entity::users::Column::Username.eq(&username))
                        //     .all(txn)
                        //     .await?;

                        // let addresses = accounts
                        //     .into_iter()
                        //     .map(|(account, _)| account.address)
                        //     .collect::<Vec<_>>();

                        // query = query.filter(
                        //     entity::transfers::Column::FromAddress
                        //         .is_in(&addresses)
                        //         .or(entity::transfers::Column::ToAddress.is_in(&addresses)),
                        // );

                        // Use subquery to check if transfer involves any account owned by the user
                        let account_addresses_subquery = entity::accounts::Entity::find()
                            .select_only()
                            .column(entity::accounts::Column::Address)
                            .join(
                                JoinType::InnerJoin,
                                entity::accounts::Relation::AccountUser.def(),
                            )
                            .join(
                                JoinType::InnerJoin,
                                entity::accounts_users::Relation::User.def(),
                            )
                            .filter(entity::users::Column::Username.eq(&username));

                        query =
                            query.filter(
                                Condition::any()
                                    .add(entity::transfers::Column::FromAddress.in_subquery(
                                        account_addresses_subquery.clone().as_query().to_owned(),
                                    ))
                                    .add(entity::transfers::Column::ToAddress.in_subquery(
                                        account_addresses_subquery.as_query().to_owned(),
                                    )),
                            );
                    }
                    Ok(query)
                })
            },
        )
        .await
    }
}

impl CursorFilter<SortBy, TransferCursor> for Select<entity::transfers::Entity> {
    fn cursor_filter(self, sort: &SortBy, cursor: &TransferCursor) -> Self {
        match sort {
            SortBy::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::transfers::Column::BlockHeight.gt(cursor.block_height as i64))
                    .add(
                        entity::transfers::Column::BlockHeight
                            .gte(cursor.block_height as i64)
                            .and(entity::transfers::Column::Idx.gt(cursor.idx)),
                    ),
            ),
            SortBy::BlockHeightDesc => self.filter(
                Condition::any()
                    .add(entity::transfers::Column::BlockHeight.lt(cursor.block_height as i64))
                    .add(
                        entity::transfers::Column::BlockHeight
                            .lte(cursor.block_height as i64)
                            .and(entity::transfers::Column::Idx.lt(cursor.idx)),
                    ),
            ),
        }
    }
}

impl CursorOrder<SortBy> for Select<entity::transfers::Entity> {
    fn cursor_order(self, sort: SortBy) -> Self {
        let order: Order = sort.into();

        self.order_by(entity::transfers::Column::BlockHeight, order.clone())
            .order_by(entity::transfers::Column::Idx, order)
    }
}
