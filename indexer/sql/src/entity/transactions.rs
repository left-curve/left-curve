#[cfg(feature = "async-graphql")]
use {
    crate::dataloaders::transaction_grug::FileTransactionDataLoader,
    crate::dataloaders::{
        transaction_events::TransactionEventsDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    async_graphql::{ComplexObject, Context, Error, Result, SimpleObject, dataloader::DataLoader},
    grug_types::{JsonSerExt, Tx, TxOutcome},
};
use {
    crate::entity::OrderByBlocks,
    grug_types::FlatCategory,
    sea_orm::{Order, QueryOrder, entity::prelude::*},
    serde::Deserialize,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Hash, Deserialize)]
#[sea_orm(table_name = "transactions")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Transaction"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    pub created_at: DateTime,
    pub block_height: i64,
    pub transaction_type: FlatCategory,
    pub transaction_idx: i32,
    pub sender: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    #[sea_orm(column_type = "JsonBinary")]
    pub credential: Json,
    pub hash: String,
    pub has_succeeded: bool,
    pub error_message: Option<String>,
    pub gas_wanted: i64,
    pub gas_used: i64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Nested Events from this transaction, from block on-disk caching
    async fn nested_events(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let (_, outcome) = load_tx_from_file(self, ctx).await?;

        Ok(Some(outcome.events.to_json_string()?))
    }

    /// Flatten events from the indexer
    async fn flatten_events(&self, ctx: &Context<'_>) -> Result<Vec<super::events::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<TransactionEventsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn messages(&self, ctx: &Context<'_>) -> Result<Vec<super::messages::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<TransactionMessagesDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }
}

#[cfg(feature = "async-graphql")]
async fn load_tx_from_file(tx: &Model, ctx: &Context<'_>) -> Result<(Tx, TxOutcome)> {
    let loader = ctx.data_unchecked::<DataLoader<FileTransactionDataLoader>>();

    loader
        .load_one(tx.clone())
        .await?
        .ok_or(Error::new(format!("transaction not found: {}", tx.hash)))
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl OrderByBlocks for Select<Entity> {
    fn order_by_blocks_desc(&self) -> Self {
        self.clone()
            .order_by(Column::BlockHeight, Order::Desc)
            .order_by(Column::TransactionIdx, Order::Desc)
    }

    fn order_by_blocks_asc(&self) -> Self {
        self.clone()
            .order_by(Column::BlockHeight, Order::Asc)
            .order_by(Column::TransactionIdx, Order::Asc)
    }
}
