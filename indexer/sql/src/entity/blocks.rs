#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Result, SimpleObject, dataloader::DataLoader};
use {
    crate::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
    },
    sea_orm::{QueryOrder, entity::prelude::*},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default, Hash)]
#[sea_orm(table_name = "blocks")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Block"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    pub created_at: DateTime,
    #[sea_orm(unique)]
    pub block_height: i64,
    pub hash: String,
    pub app_hash: String,
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub transactions_count: i32,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Transactions order isn't guaranteed, check `transactionIdx`
    async fn transactions(&self, ctx: &Context<'_>) -> Result<Vec<super::transactions::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockTransactionsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn flatten_events(&self, ctx: &Context<'_>) -> Result<Vec<super::events::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockEventsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub async fn latest_block_height<C>(db: &C) -> Result<Option<i64>, DbErr>
where
    C: ConnectionTrait,
{
    Ok(Entity::find()
        .order_by_desc(Column::BlockHeight)
        .one(db)
        .await?
        .map(|block| block.block_height))
}
