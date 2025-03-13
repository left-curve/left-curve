use {
    crate::{
        context::Context,
        graphql::types::{self, block::Block},
    },
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::{self, prelude::Blocks},
    sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select},
    serde::{Deserialize, Serialize},
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

impl From<types::block::Block> for BlockCursor {
    fn from(block: types::block::Block) -> Self {
        Self {
            block_height: block.block_height,
        }
    }
}

pub type BlockCursorType = OpaqueCursor<BlockCursor>;

static MAX_BLOCKS: u64 = 100;

#[derive(Default, Debug)]
pub struct BlockQuery {}

#[Object]
impl BlockQuery {
    /// Get a block
    async fn block(
        &self,
        ctx: &async_graphql::Context<'_>,
        height: Option<u64>,
    ) -> Result<Option<types::block::Block>> {
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

        Ok(query.one(&app_ctx.db).await?.map(|block| block.into()))
    }

    /// Get a block
    async fn blocks(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<Connection<BlockCursorType, Block, EmptyFields, EmptyFields>> {
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
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, &after);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_BLOCKS);

                        query = query.limit(limit + 1);
                    },
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, &before);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_BLOCKS);

                        query = query.limit(limit + 1);
                    },
                    _ => unreachable!(),
                }

                match sort_by {
                    SortBy::BlockHeightAsc => {
                        query = query.order_by(entity::blocks::Column::BlockHeight, Order::Asc)
                    },
                    SortBy::BlockHeightDesc => {
                        query = query.order_by(entity::blocks::Column::BlockHeight, Order::Desc)
                    },
                }

                let mut blocks: Vec<types::block::Block> = query
                    .all(&app_ctx.db)
                    .await?
                    .into_iter()
                    .map(|block| block.into())
                    .collect::<Vec<_>>();

                if has_before {
                    blocks.reverse();
                }

                let mut has_more = false;
                if blocks.len() > limit as usize {
                    blocks.pop();
                    has_more = true;
                }

                let mut connection = Connection::new(first.unwrap_or_default() > 0, has_more);
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

fn apply_filter(query: Select<Blocks>, sort_by: SortBy, after: &BlockCursor) -> Select<Blocks> {
    match sort_by {
        SortBy::BlockHeightAsc => {
            query.filter(entity::blocks::Column::BlockHeight.lt(after.block_height))
        },
        SortBy::BlockHeightDesc => {
            query.filter(entity::blocks::Column::BlockHeight.gt(after.block_height))
        },
    }
}
