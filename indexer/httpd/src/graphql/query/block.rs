use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::{self, OrderByBlocks},
    sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::cmp,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "BlockSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockCursor {
    block_height: u64,
}

impl From<entity::blocks::Model> for BlockCursor {
    fn from(block: entity::blocks::Model) -> Self {
        Self {
            block_height: block.block_height as u64,
        }
    }
}

pub type BlockCursorType = OpaqueCursor<BlockCursor>;
type Blocks = entity::blocks::Model;

const MAX_BLOCKS: u64 = 100;

#[derive(Default, Debug)]
pub struct BlockQuery {}

#[Object]
impl BlockQuery {
    /// Get a block
    async fn block(
        &self,
        ctx: &async_graphql::Context<'_>,
        height: Option<u64>,
    ) -> Result<Option<entity::blocks::Model>> {
        let app_ctx = ctx.data::<Context>()?;

        let mut query = entity::blocks::Entity::find();

        match height {
            Some(height) => {
                query = query.filter(entity::blocks::Column::BlockHeight.eq(height as i64));
            },
            None => {
                query = query.order_by(entity::blocks::Column::BlockHeight, Order::Desc);
            },
        }

        Ok(query.one(&app_ctx.db).await?)
    }

    /// Get a block
    async fn blocks(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<Connection<BlockCursorType, Blocks, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;

        query_with::<BlockCursorType, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = entity::blocks::Entity::find();
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
                        limit = cmp::min(first as u64, MAX_BLOCKS);
                        query = query.limit(limit + 1);
                    },
                    (first, Some(after), None, None) => {
                        query = apply_filter(query, sort_by, &after);

                        limit = cmp::min(first.unwrap_or(0) as u64, MAX_BLOCKS);
                        query = query.limit(limit + 1);

                        has_previous_page = true;
                    },

                    (None, None, Some(last), None) => {
                        limit = cmp::min(last as u64, MAX_BLOCKS);
                        query = query.limit(limit + 1);
                    },

                    (None, None, last, Some(before)) => {
                        query = match &sort_by {
                            SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
                        };

                        limit = cmp::min(last.unwrap_or(0) as u64, MAX_BLOCKS);
                        query = query.limit(limit + 1);

                        has_next_page = true;
                    },

                    (None, None, None, None) => {
                        limit = MAX_BLOCKS;
                        query = query.limit(MAX_BLOCKS + 1);
                    }

                    _ => {
                        return Err(async_graphql::Error::new(
                            "Unexpected combination of pagination parameters, should use first with after or last with before",
                        ));
                    },
                }

                let mut blocks = query.all(&app_ctx.db).await?;

                if blocks.len() > limit as usize {
                    blocks.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    blocks.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.edges.extend(blocks.into_iter().map(|block| {
                    Edge::with_additional_fields(
                        OpaqueCursor(block.clone().into()),
                        block,
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
    query: Select<entity::blocks::Entity>,
    sort_by: SortBy,
    after: &BlockCursor,
) -> Select<entity::blocks::Entity> {
    match sort_by {
        SortBy::BlockHeightAsc => {
            query.filter(entity::blocks::Column::BlockHeight.gt(after.block_height))
        },
        SortBy::BlockHeightDesc => {
            query.filter(entity::blocks::Column::BlockHeight.lt(after.block_height))
        },
    }
}
