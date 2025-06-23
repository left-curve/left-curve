use {
    crate::{
        context::Context,
        graphql::query::pagination::{CursorFilter, CursorOrder, Reversible, paginate_models},
    },
    async_graphql::{types::connection::*, *},
    indexer_sql::entity,
    sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, Select},
    serde::{Deserialize, Serialize},
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Default)]
#[graphql(name = "BlockSortBy")]
pub enum SortBy {
    BlockHeightAsc,
    #[default]
    BlockHeightDesc,
}

impl Reversible for SortBy {
    fn rev(&self) -> Self {
        match self {
            SortBy::BlockHeightAsc => SortBy::BlockHeightDesc,
            SortBy::BlockHeightDesc => SortBy::BlockHeightAsc,
        }
    }
}

impl From<SortBy> for Order {
    fn from(sort_by: SortBy) -> Self {
        match sort_by {
            SortBy::BlockHeightAsc => Order::Asc,
            SortBy::BlockHeightDesc => Order::Desc,
        }
    }
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

#[derive(Default, Debug)]
pub struct BlockQuery {}

#[Object]
impl BlockQuery {
    /// Get a block.
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

    /// Get paginated blocks.
    async fn blocks(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Cursor based pagination")] after: Option<String>,
        #[graphql(desc = "Cursor based pagination")] before: Option<String>,
        #[graphql(desc = "Cursor based pagination")] first: Option<i32>,
        #[graphql(desc = "Cursor based pagination")] last: Option<i32>,
        sort_by: Option<SortBy>,
    ) -> Result<
        Connection<OpaqueCursor<BlockCursor>, entity::blocks::Model, EmptyFields, EmptyFields>,
    > {
        let app_ctx = ctx.data::<Context>()?;

        paginate_models::<BlockCursor, entity::blocks::Entity, SortBy>(
            app_ctx,
            after,
            before,
            first,
            last,
            sort_by,
            100,
            |query, _| Box::pin(async move { Ok(query) }),
        )
        .await
    }
}

impl CursorFilter<SortBy, BlockCursor> for Select<entity::blocks::Entity> {
    fn cursor_filter(self, sort: &SortBy, cursor: &BlockCursor) -> Self {
        match sort {
            SortBy::BlockHeightAsc => {
                self.filter(entity::blocks::Column::BlockHeight.gt(cursor.block_height))
            },
            SortBy::BlockHeightDesc => {
                self.filter(entity::blocks::Column::BlockHeight.lt(cursor.block_height))
            },
        }
    }
}

impl CursorOrder<SortBy> for Select<entity::blocks::Entity> {
    fn cursor_order(self, sort: SortBy) -> Self {
        let order: Order = sort.into();
        self.order_by(entity::blocks::Column::BlockHeight, order)
    }
}
