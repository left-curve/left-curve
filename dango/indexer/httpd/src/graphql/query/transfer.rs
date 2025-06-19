use {
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity::{self, OrderByBlocks},
    indexer_httpd::context::Context,
    sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "TransferSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
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

pub type TransferCursorType = OpaqueCursor<TransferCursor>;

static MAX_TRANSFERS: u64 = 100;

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
    ) -> Result<Connection<TransferCursorType, entity::transfers::Model, EmptyFields, EmptyFields>>
    {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<TransferCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::transfers::Entity::find();
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
                        limit = cmp::min(first as u64, MAX_TRANSFERS);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_TRANSFERS);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_TRANSFERS);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_TRANSFERS);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_TRANSFERS;
                        query = query.limit(MAX_TRANSFERS + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                if let Some(block_height) = block_height {
                    query = query
                        .filter(entity::transfers::Column::BlockHeight.eq(block_height as i64));
                }

                if let Some(from_address) = from_address {
                    query = query.filter(entity::transfers::Column::FromAddress.eq(from_address));
                }

                if let Some(to_address) = to_address {
                    query = query.filter(entity::transfers::Column::ToAddress.eq(to_address));
                }

                if let Some(username) = username {
                    let accounts = entity::accounts::Entity::find()
                        .find_also_related(entity::users::Entity)
                        .filter(entity::users::Column::Username.eq(username))
                        .all(&app_ctx.db)
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

                let mut transfers = query.all(&app_ctx.db).await?;

                if transfers.len() > limit as usize {
                    transfers.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    transfers.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection
                    .edges
                    .extend(transfers.into_iter().map(|transfer| {
                        Edge::with_additional_fields(
                            OpaqueCursor(transfer.clone().into()),
                            transfer,
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
    query: Select<entity::transfers::Entity>,
    sort_by: SortBy,
    after: &TransferCursor,
) -> Select<entity::transfers::Entity> {
    query.filter(match sort_by {
        SortBy::BlockHeightDesc => Condition::any()
            .add(entity::transfers::Column::BlockHeight.lt(after.block_height as i64))
            .add(
                entity::transfers::Column::BlockHeight
                    .lte(after.block_height as i64)
                    .and(entity::transfers::Column::Idx.lt(after.idx)),
            ),
        SortBy::BlockHeightAsc => Condition::any()
            .add(entity::transfers::Column::BlockHeight.gt(after.block_height as i64))
            .add(
                entity::transfers::Column::BlockHeight
                    .gte(after.block_height as i64)
                    .and(entity::transfers::Column::Idx.gt(after.idx)),
            ),
    })
}
