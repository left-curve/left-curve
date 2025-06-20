use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity,
    indexer_httpd::{
        context::Context,
        graphql::query::pagination::{CursorFilter, SortByEnum, paginate_models},
    },
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "TransferSortBy")]
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
        let db = app_ctx.db.clone();

        paginate_models::<TransferCursor, entity::transfers::Entity, SortBy>(
            app_ctx,
            after,
            before,
            first,
            last,
            sort_by,
            100,
            |query| {
                Box::pin(async move {
                    let mut query = query;

                    if let Some(block_height) = block_height {
                        query = query
                            .filter(entity::transfers::Column::BlockHeight.eq(block_height as i64));
                    }

                    if let Some(from_address) = from_address {
                        query =
                            query.filter(entity::transfers::Column::FromAddress.eq(from_address));
                    }

                    if let Some(to_address) = to_address {
                        query = query.filter(entity::transfers::Column::ToAddress.eq(to_address));
                    }

                    if let Some(username) = username {
                        let accounts = entity::accounts::Entity::find()
                            .find_also_related(entity::users::Entity)
                            .filter(entity::users::Column::Username.eq(username))
                            .all(&db)
                            .await?;

                        let addresses = accounts
                            .into_iter()
                            .map(|(account, _)| account.address)
                            .collect::<Vec<_>>();

                        query = query.filter(
                            entity::transfers::Column::FromAddress
                                .is_in(&addresses)
                                .or(entity::transfers::Column::ToAddress.is_in(&addresses)),
                        );
                    }

                    Ok(query)
                })
            },
        )
        .await
    }
}

impl CursorFilter<TransferCursor> for Select<entity::transfers::Entity> {
    fn apply_cursor_filter(self, sort_by: SortByEnum, cursor: &TransferCursor) -> Self {
        match sort_by {
            SortByEnum::BlockHeightAsc => self.filter(
                Condition::any()
                    .add(entity::transfers::Column::BlockHeight.gt(cursor.block_height as i64))
                    .add(
                        entity::transfers::Column::BlockHeight
                            .gte(cursor.block_height as i64)
                            .and(entity::transfers::Column::Idx.gt(cursor.idx)),
                    ),
            ),
            SortByEnum::BlockHeightDesc => self.filter(
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
