use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity,
    indexer_httpd::{
        context::Context,
        graphql::query::pagination::{CursorFilter, paginate_models},
    },
    sea_orm::{
        ColumnTrait, Condition, JoinType, Order, QueryFilter, QuerySelect, RelationTrait, Select,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "AccountSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
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
pub struct AccountCursor {
    created_block_height: u64,
    address: String,
}

impl From<entity::accounts::Model> for AccountCursor {
    fn from(account: entity::accounts::Model) -> Self {
        Self {
            created_block_height: account.created_block_height as u64,
            address: account.address,
        }
    }
}

#[derive(Default, Debug)]
pub struct AccountQuery {}

#[Object]
impl AccountQuery {
    /// Get paginated accounts
    async fn accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
        // The block height at which the account was created
        block_height: Option<u64>,
        username: Option<String>,
        address: Option<String>,
    ) -> Result<
        Connection<OpaqueCursor<AccountCursor>, entity::accounts::Model, EmptyFields, EmptyFields>,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<AccountCursor, entity::accounts::Entity, SortBy>(
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
                            entity::accounts::Column::CreatedBlockHeight.eq(block_height as i64),
                        );
                    }

                    if let Some(address) = address {
                        query = query.filter(entity::accounts::Column::Address.eq(&address));
                    }

                    if let Some(username) = username {
                        query = query
                            .join(
                                JoinType::InnerJoin,
                                entity::accounts::Relation::AccountUser.def(),
                            )
                            .join(
                                JoinType::InnerJoin,
                                entity::accounts_users::Relation::User.def(),
                            )
                            .filter(entity::users::Column::Username.eq(&username));
                    }

                    Ok(query)
                })
            },
        )
        .await
    }
}

impl CursorFilter<AccountCursor> for Select<entity::accounts::Entity> {
    fn cursor_filter(self, order: Order, cursor: &AccountCursor) -> Self {
        match order {
            Order::Asc => self.filter(
                Condition::any()
                    .add(
                        entity::accounts::Column::CreatedBlockHeight
                            .gt(cursor.created_block_height as i64),
                    )
                    .add(
                        entity::accounts::Column::CreatedBlockHeight
                            .gte(cursor.created_block_height as i64)
                            .and(entity::accounts::Column::Address.gt(&cursor.address)),
                    ),
            ),
            Order::Desc => self.filter(
                Condition::any()
                    .add(
                        entity::accounts::Column::CreatedBlockHeight
                            .lt(cursor.created_block_height as i64),
                    )
                    .add(
                        entity::accounts::Column::CreatedBlockHeight
                            .lte(cursor.created_block_height as i64)
                            .and(entity::accounts::Column::Address.lt(&cursor.address)),
                    ),
            ),
            Order::Field(_) => self,
        }
    }
}
