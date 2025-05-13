use {
    crate::graphql::types::{self, account::Account},
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity::{self, prelude::Accounts},
    indexer_httpd::context::Context,
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountCursor {
    created_block_height: u64,
    // username: String,
}

impl From<types::account::Account> for AccountCursor {
    fn from(account: types::account::Account) -> Self {
        Self {
            created_block_height: account.created_block_height,
            // username: account.username,
        }
    }
}

pub type AccountCursorType = OpaqueCursor<AccountCursor>;

static MAX_ACCOUNTS: u64 = 100;

#[derive(Default, Debug)]
pub struct AccountQuery {}

#[Object]
impl AccountQuery {
    async fn accounts(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
        // The block height of created account
        block_height: Option<u64>,
        username: Option<String>,
        address: Option<String>,
    ) -> Result<Connection<AccountCursorType, Account, EmptyFields, EmptyFields>> {
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
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_ACCOUNTS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_ACCOUNTS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
                }

                if let Some(block_height) = block_height {
                    query = query.filter(
                        entity::accounts::Column::CreatedBlockHeight.eq(block_height as i64),
                    );
                }

                if let Some(address) = address {
                    query = query.filter(entity::accounts::Column::Address.eq(&address));
                }

                if let Some(_username) = username {
                    // query = query.filter(entity::accounts::Column::Username.eq(&username));
                }

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query =
                            query.order_by(entity::accounts::Column::CreatedBlockHeight, Order::Asc)
                        // .order_by(entity::accounts::Column::Username, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query
                            .order_by(entity::accounts::Column::CreatedBlockHeight, Order::Desc)
                        // .order_by(entity::accounts::Column::Username, Order::Desc)
                    },
                }

                let mut accounts: Vec<types::account::Account> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|account| account.into())
                    .collect::<Vec<_>>();

                if has_before {
                    accounts.reverse();
                }

                let mut has_more = false;
                if accounts.len() > limit as usize {
                    accounts.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
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
    query: Select<Accounts>,
    sort_by: SortBy,
    after: &AccountCursor,
) -> Select<Accounts> {
    query.filter(match sort_by {
        SortBy::BlockHeightAsc => Condition::any()
            .add(entity::accounts::Column::CreatedBlockHeight.lt(after.created_block_height as i64))
            .add(
                entity::accounts::Column::CreatedBlockHeight
                    .eq(after.created_block_height as i64)
                    // .and(entity::accounts::Column::Username.lt(&after.username)),
            ),
        SortBy::BlockHeightDesc => Condition::any()
            .add(entity::accounts::Column::CreatedBlockHeight.gt(after.created_block_height as i64))
            .add(
                entity::accounts::Column::CreatedBlockHeight
                    .eq(after.created_block_height as i64)
                    // .and(entity::accounts::Column::Username.gt(&after.username)),
            ),
    })
}
