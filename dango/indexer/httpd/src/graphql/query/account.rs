use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity::{self, OrderByBlocks},
    indexer_httpd::context::Context,
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "AccountSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
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

pub type AccountCursorType = OpaqueCursor<AccountCursor>;

const MAX_ACCOUNTS: u64 = 100;

#[derive(Default, Debug)]
pub struct AccountQuery {}

#[Object]
impl AccountQuery {
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
    ) -> Result<Connection<AccountCursorType, entity::accounts::Model, EmptyFields, EmptyFields>>
    {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<AccountCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::accounts::Entity::find();
                let sort_by = sort_by.unwrap_or_default();
                let limit;

                let mut has_next_page = false;
                let mut has_previous_page = false;

                // see https://github.com/SeaQL/sea-orm/issues/2220
                // order *must* be before `find_with_related`
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
                        limit = cmp::min(first as u64, MAX_ACCOUNTS);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_ACCOUNTS);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_ACCOUNTS);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_ACCOUNTS);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_ACCOUNTS;
                        query = query.limit(MAX_ACCOUNTS + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                if let Some(block_height) = block_height {
                    query = query.filter(
                        entity::accounts::Column::CreatedBlockHeight.eq(block_height as i64),
                    );
                }

                if let Some(address) = address {
                    query = query.filter(entity::accounts::Column::Address.eq(&address));
                }

                let mut query = query.find_with_related(entity::users::Entity);

                if let Some(username) = username {
                    query = query.filter(entity::users::Column::Username.eq(&username));
                }

                let mut accounts = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|(account, _)| account)
                    .collect::<Vec<_>>();

                if accounts.len() > limit as usize {
                    accounts.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    accounts.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.edges.extend(accounts.into_iter().map(|account| {
                    Edge::with_additional_fields(
                        OpaqueCursor(account.clone().into()),
                        account,
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
    query: Select<entity::accounts::Entity>,
    sort_by: SortBy,
    after: &AccountCursor,
) -> Select<entity::accounts::Entity> {
    query.filter(match sort_by {
        SortBy::BlockHeightDesc => Condition::any()
            .add(entity::accounts::Column::CreatedBlockHeight.lt(after.created_block_height as i64))
            .add(
                entity::accounts::Column::CreatedBlockHeight
                    .lte(after.created_block_height as i64)
                    .and(entity::accounts::Column::Address.lt(&after.address)),
            ),
        SortBy::BlockHeightAsc => Condition::any()
            .add(entity::accounts::Column::CreatedBlockHeight.gt(after.created_block_height as i64))
            .add(
                entity::accounts::Column::CreatedBlockHeight
                    .gte(after.created_block_height as i64)
                    .and(entity::accounts::Column::Address.gt(&after.address)),
            ),
    })
}
