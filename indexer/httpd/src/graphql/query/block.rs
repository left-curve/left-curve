use {
    crate::context::Context,
    async_graphql::{types::connection::*, *},
    indexer_sql::entity::{self, OrderByBlocks},
    sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Select},
    serde::{Deserialize, Serialize},
    std::{cmp, fmt::Display},
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

impl CursorType for BlockCursor {
    type Error = serde_json::Error;

    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(s)
    }

    fn encode_cursor(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl From<entity::blocks::Model> for BlockCursor {
    fn from(block: entity::blocks::Model) -> Self {
        Self {
            block_height: block.block_height as u64,
        }
    }
}

pub type BlockCursorType = OpaqueCursor<BlockCursor>;

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
    ) -> Result<Connection<BlockCursorType, entity::blocks::Model, EmptyFields, EmptyFields>> {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<BlockCursor, entity::blocks::Entity>(
            app_ctx, after, before, first, last, sort_by,
        )
        .await

        // query_with::<BlockCursorType, _, _, _, _>(
        //     after,
        //     before,
        //     first,
        //     last,
        //     |after, before, first, last| async move {
        //         let mut query = entity::blocks::Entity::find();
        //         let sort_by = sort_by.unwrap_or_default();
        //         let limit;

        //         let mut has_next_page = false;
        //         let mut has_previous_page = false;

        //         match (last, sort_by) {
        //             (None, SortBy::BlockHeightAsc) | (Some(_), SortBy::BlockHeightDesc) => {
        //                 query = query.order_by_blocks_asc()
        //             },
        //             (None, SortBy::BlockHeightDesc) | (Some(_), SortBy::BlockHeightAsc) => {
        //                 query = query.order_by_blocks_desc()
        //             },
        //         }

        //         match (first, after, last, before) {
        //             (Some(first), None, None, None) => {
        //                 limit = cmp::min(first as u64, MAX_BLOCKS);
        //                 query = query.limit(limit + 1);
        //             },
        //             (first, Some(after), None, None) => {
        //                 query = apply_filter(query, sort_by, &after);

        //                 limit = cmp::min(first.unwrap_or(0) as u64, MAX_BLOCKS);
        //                 query = query.limit(limit + 1);

        //                 has_previous_page = true;
        //             },

        //             (None, None, Some(last), None) => {
        //                 limit = cmp::min(last as u64, MAX_BLOCKS);
        //                 query = query.limit(limit + 1);
        //             },

        //             (None, None, last, Some(before)) => {
        //                 query = match &sort_by {
        //                     SortBy::BlockHeightAsc =>  apply_filter(query, SortBy::BlockHeightDesc, &before),
        //                     SortBy::BlockHeightDesc => apply_filter(query, SortBy::BlockHeightAsc, &before),
        //                 };

        //                 limit = cmp::min(last.unwrap_or(0) as u64, MAX_BLOCKS);
        //                 query = query.limit(limit + 1);

        //                 has_next_page = true;
        //             },

        //             (None, None, None, None) => {
        //                 limit = MAX_BLOCKS;
        //                 query = query.limit(MAX_BLOCKS + 1);
        //             }

        //             _ => {
        //                 return Err(async_graphql::Error::new(
        //                     "Unexpected combination of pagination parameters, should use first with after or last with before",
        //                 ));
        //             },
        //         }

        //         let mut blocks = query.all(&app_ctx.db).await?;

        //         if blocks.len() > limit as usize {
        //             blocks.pop();
        //             if last.is_some() {
        //                 has_previous_page = true;
        //             } else {
        //                 has_next_page = true;
        //             }
        //         }

        //         if last.is_some() {
        //             blocks.reverse();
        //         }

        //         let mut connection = Connection::new(has_previous_page, has_next_page);
        //         connection.edges.extend(blocks.into_iter().map(|block| {
        //             Edge::with_additional_fields(
        //                 OpaqueCursor(block.clone().into()),
        //                 block,
        //                 EmptyFields,
        //             )
        //         }));

        //         Ok::<_, async_graphql::Error>(connection)
        //     },
        // )
        // .await
    }
}

pub trait CursorFilter<C> {
    fn apply_cursor_filter(self, sort_by: SortBy, cursor: &C) -> Self;
}

impl CursorFilter<BlockCursor> for Select<entity::blocks::Entity> {
    fn apply_cursor_filter(self, sort_by: SortBy, cursor: &BlockCursor) -> Self {
        match sort_by {
            SortBy::BlockHeightAsc => {
                self.filter(entity::blocks::Column::BlockHeight.gt(cursor.block_height))
            },
            SortBy::BlockHeightDesc => {
                self.filter(entity::blocks::Column::BlockHeight.lt(cursor.block_height))
            },
        }
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

async fn paginate_models<C, E>(
    app_ctx: &Context,
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    sort_by: Option<SortBy>,
) -> Result<Connection<OpaqueCursor<C>, E::Model, EmptyFields, EmptyFields>>
where
    C: CursorType + Send + Sync + Serialize + serde::de::DeserializeOwned,
    <C as CursorType>::Error: Display + Send + Sync + 'static,
    E: EntityTrait + Send + Sync,
    <E as EntityTrait>::Model: async_graphql::OutputType,
    Select<E>: OrderByBlocks + CursorFilter<C>,
    C: std::convert::From<<E as EntityTrait>::Model>,
{
    query_with::<C, _, _, _, _>(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let mut query = E::find();
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
                        query = query.apply_cursor_filter(sort_by, &after);

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
                            SortBy::BlockHeightAsc =>  query.apply_cursor_filter(SortBy::BlockHeightDesc, &before),
                            SortBy::BlockHeightDesc => query.apply_cursor_filter(SortBy::BlockHeightAsc, &before),
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

                let mut models = query.all(&app_ctx.db).await?;

                if models.len() > limit as usize {
                    models.pop();
                    if last.is_some() {
                        has_previous_page = true;
                    } else {
                        has_next_page = true;
                    }
                }

                if last.is_some() {
                    models.reverse();
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.edges.extend(models.into_iter().map(|model| {
                    Edge::with_additional_fields(
                        OpaqueCursor(model.clone().into()),
                        model,
                        EmptyFields,
                    )
                }));

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
}
