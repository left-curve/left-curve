use {
    crate::{
        context::Context,
        graphql::types::{self, block::Block},
    },
    async_graphql::{types::connection::*, *},
    // grug_types::{JsonDeExt, JsonSerExt, StdResult},
    indexer_sql::entity::{self, prelude::Blocks},
    sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select},
    // std::fmt::Display,
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
pub struct BlockCursor(u64);

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
        height: u64,
    ) -> Result<Option<types::block::Block>> {
        let app_ctx = ctx.data::<Context>()?;

        Ok(entity::blocks::Entity::find()
            .filter(entity::blocks::Column::BlockHeight.eq(height as i64))
            .one(&app_ctx.db)
            .await?
            .map(|block| block.into()))
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

        query_with(
            after,
            before,
            first,
            last,
            |after: Option<BlockCursorType>, before: Option<BlockCursorType>, first, last| async move {
                let mut query = entity::blocks::Entity::find();
                let sort_by = sort_by.unwrap_or_default();
                let limit;
                let has_before = before.is_some();

                match (after, before, first, last) {
                    (after, None, first, None) => {
                        if let Some(after) = after {
                            query = apply_filter(query, sort_by, after.0);
                        }

                        limit = first.map(|x| x as u64).unwrap_or(MAX_BLOCKS);

                        query = query.limit(limit);
                    }
                    (None, before, None, last) => {
                        if let Some(before) = before {
                            query = apply_filter(query, sort_by, before.0);
                        }

                        limit = last.map(|x| x as u64).unwrap_or(MAX_BLOCKS);

                        query = query.limit(limit);
                    }
                    _ => unreachable!()
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
                connection.edges.extend(
                    blocks
                    .into_iter()
                    .map(|block|
                        Edge::with_additional_fields(
                            OpaqueCursor(BlockCursor(block.block_height)),
                            block,
                            EmptyFields
                        )
                    )
                );
                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
}

fn apply_filter(query: Select<Blocks>, sort_by: SortBy, after: BlockCursor) -> Select<Blocks> {
    match sort_by {
        SortBy::BlockHeightAsc => {
            query.filter(entity::blocks::Column::BlockHeight.lt(after.0 as i64))
        },
        SortBy::BlockHeightDesc => {
            query.filter(entity::blocks::Column::BlockHeight.gt(after.0 as i64))
        },
    }
}
