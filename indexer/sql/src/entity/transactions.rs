#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Error, Result, SimpleObject, dataloader::DataLoader};
use {
    crate::dataloaders::{
        transaction_events::TransactionEventsDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    serde::Deserialize,
};

use {
    grug_types::{FlatCategory, JsonSerExt, Tx, TxOutcome},
    sea_orm::entity::prelude::*,
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

    // async fn data(&self, ctx: &Context<'_>) -> Result<String> {
    //     let (tx, _) = load_tx_from_file(self, ctx).await?;
    //     Ok(tx.data.to_json_string()?)
    // }

    // async fn credential(&self, ctx: &Context<'_>) -> Result<String> {
    //     let (tx, _) = load_tx_from_file(self, ctx).await?;
    //     Ok(tx.credential.to_json_string()?)
    // }
}

#[cfg(feature = "async-graphql")]
async fn load_tx_from_file(tx: &Model, ctx: &Context<'_>) -> Result<(Tx, TxOutcome)> {
    use crate::dataloaders::transaction_grug::FileTransactionDataLoader;

    let loader = ctx.data_unchecked::<DataLoader<FileTransactionDataLoader>>();
    loader
        .load_one(tx.clone())
        .await?
        .ok_or(Error::new(format!("Transaction not found: {}", tx.hash)))
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
