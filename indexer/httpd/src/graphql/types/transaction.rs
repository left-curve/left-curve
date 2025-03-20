use {
    async_graphql::*,
    chrono::{DateTime, TimeZone, Utc},
    indexer_sql::{block_to_index::BlockToIndex, entity},
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Transaction {
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
    async fn nested_events(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let block_filename = app_ctx.indexer_path.block_path(self.block_height);
        let tx_idx = self.transaction_idx as usize;

        let data = BlockToIndex::load_from_disk(block_filename)?;

        data.block_outcome
            .tx_outcomes
            .get(tx_idx)
            .map(|tx| Ok(serde_json::to_string(&tx.events)?))
            .transpose()
    }
}
