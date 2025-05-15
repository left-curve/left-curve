use {
    super::{event::Event, message::Message},
    crate::graphql::dataloader::{
        file_transaction::FileTranscationDataLoader,
        transaction_events::TransactionEventsDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    async_graphql::{dataloader::DataLoader, *},
    chrono::{DateTime, TimeZone, Utc},
    grug_types::{JsonSerExt, Tx, TxOutcome},
    indexer_sql::entity,
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Transaction {
    #[graphql(skip)]
    pub id: uuid::Uuid,
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
    pub transaction_type: Category,
    pub transaction_idx: u32,
    pub sender: String,
    pub hash: String,
    pub has_succeeded: bool,
    pub error_message: Option<String>,
    pub gas_wanted: i64,
    pub gas_used: i64,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq, Debug, Deserialize, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Category {
    #[default]
    Cron,
    Tx,
}

impl From<entity::events::TransactionType> for Category {
    fn from(status: entity::events::TransactionType) -> Category {
        match status {
            entity::events::TransactionType::Cron => Category::Cron,
            entity::events::TransactionType::Tx => Category::Tx,
        }
    }
}

impl From<entity::transactions::Model> for Transaction {
    fn from(item: entity::transactions::Model) -> Self {
        Self {
            id: item.id,
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            transaction_type: item.transaction_type.into(),
            transaction_idx: item.transaction_idx as u32,
            sender: item.sender.clone(),
            hash: item.hash.clone(),
            has_succeeded: item.has_succeeded,
            error_message: item.error_message.clone(),
            gas_wanted: item.gas_wanted,
            gas_used: item.gas_used,
        }
    }
}

#[ComplexObject]
impl Transaction {
    /// Nested Events from this transaction, from block on-disk caching
    async fn nested_events(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let (_, outcome) = load_tx_from_file(self, ctx).await?;

        Ok(Some(outcome.events.to_json_string()?))
    }

    /// Flatten events from the indexer
    async fn flatten_events(&self, ctx: &Context<'_>) -> Result<Vec<Event>> {
        let loader = ctx.data_unchecked::<DataLoader<TransactionEventsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn messages(&self, ctx: &Context<'_>) -> Result<Vec<Message>> {
        let loader = ctx.data_unchecked::<DataLoader<TransactionMessagesDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn data(&self, ctx: &Context<'_>) -> Result<String> {
        let (tx, _) = load_tx_from_file(self, ctx).await?;
        Ok(tx.data.to_json_string()?)
    }

    async fn credential(&self, ctx: &Context<'_>) -> Result<String> {
        let (tx, _) = load_tx_from_file(self, ctx).await?;
        Ok(tx.credential.to_json_string()?)
    }
}

async fn load_tx_from_file(tx: &Transaction, ctx: &Context<'_>) -> Result<(Tx, TxOutcome)> {
    let loader = ctx.data_unchecked::<DataLoader<FileTranscationDataLoader>>();
    loader
        .load_one(tx.clone())
        .await?
        .ok_or(Error::new(format!("Transaction not found: {}", tx.hash)))
}
