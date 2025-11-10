#[cfg(feature = "async-graphql")]
use {
    crate::block_to_index::BlockToIndex,
    crate::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
    },
    crate::indexer_path::IndexerPath,
    async_graphql::{ComplexObject, Context, Result, SimpleObject, dataloader::DataLoader},
    grug_types::JsonSerExt,
    grug_types::Timestamp,
};
use {
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
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::serde_iso8601")]
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
    /// Returns the block timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at).to_rfc3339_string()
    }

    /// Transactions order isn't guaranteed, check `transactionIdx`
    async fn transactions(&self, ctx: &Context<'_>) -> Result<Vec<super::transactions::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockTransactionsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn crons_outcomes(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        Ok(BlockToIndex::load_from_disk_async(
            ctx.data_unchecked::<IndexerPath>()
                .block_path(self.block_height as u64),
        )
        .await?
        .block_outcome
        .cron_outcomes
        .iter()
        .map(JsonSerExt::to_json_string)
        .collect::<Result<Vec<_>, _>>()?)
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
