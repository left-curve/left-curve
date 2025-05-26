use {
    async_graphql::{
        connection::{Connection, Edge, EmptyFields, OpaqueCursor, query_with},
        *,
    },
    dango_indexer_sql::entity::{self},
    indexer_httpd::context::Context,
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserCursor {
    created_block_height: u64,
    username: String,
}

impl From<entity::users::Model> for UserCursor {
    fn from(user: entity::users::Model) -> Self {
        Self {
            created_block_height: user.created_block_height as u64,
            username: user.username,
        }
    }
}

pub type UserCursorType = OpaqueCursor<UserCursor>;

static MAX_USERS: u64 = 100;

#[derive(Default, Debug)]
pub struct UserQuery {}

#[Object]
impl UserQuery {
    async fn user(
        &self,
        ctx: &async_graphql::Context<'_>,
        username: String,
    ) -> Result<Option<entity::users::Model>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(entity::users::Entity::find()
            .filter(entity::users::Column::Username.eq(&username))
            .one(&app_ctx.db)
            .await?)
    }

    async fn users(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        // The block height at which the user was created
        block_height: Option<u64>,
        public_key: Option<String>,
        public_key_hash: Option<String>,
    ) -> Result<Connection<UserCursorType, entity::users::Model, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<UserCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::users::Entity::find();
                let limit;
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_USERS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_USERS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
                }

                if let Some(block_height) = block_height {
                    query = query
                        .filter(entity::users::Column::CreatedBlockHeight.eq(block_height as i64));
                }

                //  sort must be before `find_with_related`
                query = query
                    .order_by(entity::users::Column::CreatedBlockHeight, Order::Desc)
                    .order_by(entity::users::Column::Username, Order::Desc);

                let mut query = query.find_with_related(entity::public_keys::Entity);

                if let Some(public_key) = public_key {
                    query = query.filter(entity::public_keys::Column::PublicKey.eq(&public_key));
                }

                if let Some(public_key_hash) = public_key_hash {
                    query = query.filter(entity::public_keys::Column::KeyHash.eq(&public_key_hash));
                }

                let mut users = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|(user, _)| user)
                    .collect::<Vec<_>>();

                if has_before {
                    users.reverse();
                }

                let mut has_more = false;
                if users.len() > limit as usize {
                    users.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
                connection.edges.extend(users.into_iter().map(|account| {
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
    query: Select<entity::users::Entity>,
    after: &UserCursor,
) -> Select<entity::users::Entity> {
    query.filter(
        Condition::any()
            .add(entity::users::Column::CreatedBlockHeight.lt(after.created_block_height as i64))
            .add(entity::users::Column::CreatedBlockHeight.eq(after.created_block_height as i64)),
    )
}
