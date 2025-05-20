use {
    crate::graphql::types::{self, transfer::Transfer},
    async_graphql::{types::connection::*, *},
    dango_indexer_sql::entity::{self},
    indexer_httpd::context::Context,
    sea_orm::{
        ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferCursor {
    block_height: u64,
    idx: i32,
}

impl From<types::transfer::Transfer> for TransferCursor {
    fn from(transfer: types::transfer::Transfer) -> Self {
        Self {
            block_height: transfer.block_height,
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
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
        // The block height of the transfer
        block_height: Option<u64>,
        // The from address of the transfer
        from_address: Option<String>,
        // The to address of the transfer
        to_address: Option<String>,
    ) -> Result<Connection<TransferCursorType, Transfer, EmptyFields, EmptyFields>> {
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
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_TRANSFERS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_TRANSFERS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
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

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query = query.order_by(entity::transfers::Column::BlockHeight, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query.order_by(entity::transfers::Column::BlockHeight, Order::Desc)
                    },
                }

                let mut transfers: Vec<types::transfer::Transfer> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|transfer| transfer.into())
                    .collect::<Vec<_>>();

                if has_before {
                    transfers.reverse();
                }

                let mut has_more = false;
                if transfers.len() > limit as usize {
                    transfers.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
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
        SortBy::BlockHeightAsc => Condition::any()
            .add(entity::transfers::Column::BlockHeight.lt(after.block_height as i64))
            .add(
                entity::transfers::Column::BlockHeight
                    .eq(after.block_height as i64)
                    .and(entity::transfers::Column::Idx.lt(after.idx)),
            ),
        SortBy::BlockHeightDesc => Condition::any()
            .add(entity::transfers::Column::BlockHeight.gt(after.block_height as i64))
            .add(
                entity::transfers::Column::BlockHeight
                    .eq(after.block_height as i64)
                    .and(entity::transfers::Column::Idx.gt(after.idx)),
            ),
    })
}
