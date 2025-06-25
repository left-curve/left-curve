#[cfg(feature = "async-graphql")]
use {
    crate::dataloaders::event_transaction::EventTransactionDataLoader,
    async_graphql::{ComplexObject, Context, Enum, Result, SimpleObject, dataloader::DataLoader},
    grug_types::Timestamp,
};
use {
    grug_types::{FlatCategory, FlatCommitmentStatus, FlatEventStatus},
    sea_orm::entity::prelude::*,
    serde::Deserialize,
};

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum CommitmentStatus {
    #[sea_orm(num_value = 0)]
    Committed,
    #[sea_orm(num_value = 1)]
    Failed,
    #[sea_orm(num_value = 2)]
    Reverted,
}

impl From<FlatCommitmentStatus> for CommitmentStatus {
    fn from(value: FlatCommitmentStatus) -> Self {
        match value {
            FlatCommitmentStatus::Committed => CommitmentStatus::Committed,
            FlatCommitmentStatus::Failed => CommitmentStatus::Failed,
            FlatCommitmentStatus::Reverted => CommitmentStatus::Reverted,
        }
    }
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq, Copy, Deserialize, Hash)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
#[cfg_attr(feature = "async-graphql", derive(Enum))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "async-graphql", graphql(rename_items = "snake_case"))]
pub enum EventStatus {
    #[sea_orm(num_value = 0)]
    Ok,
    #[sea_orm(num_value = 1)]
    Failed,
    #[sea_orm(num_value = 2)]
    NestedFailed,
    #[sea_orm(num_value = 3)]
    Handled,
}

impl From<FlatEventStatus> for EventStatus {
    fn from(value: FlatEventStatus) -> Self {
        match value {
            FlatEventStatus::Ok => EventStatus::Ok,
            FlatEventStatus::Failed(_) => EventStatus::Failed,
            FlatEventStatus::NestedFailed => EventStatus::NestedFailed,
            FlatEventStatus::Handled(_) => EventStatus::Handled,
        }
    }
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, PartialEq, Eq)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TransactionType {
    #[sea_orm(num_value = 0)]
    Cron,
    #[sea_orm(num_value = 1)]
    Tx,
}

impl From<FlatCategory> for TransactionType {
    fn from(value: FlatCategory) -> Self {
        match value {
            FlatCategory::Cron => TransactionType::Cron,
            FlatCategory::Tx => TransactionType::Tx,
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize, Hash)]
#[sea_orm(table_name = "events")]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Event"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub id: Uuid,
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub parent_id: Option<Uuid>,
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub transaction_id: Option<Uuid>,
    #[cfg_attr(
        all(feature = "async-graphql", not(feature = "testing")),
        graphql(skip)
    )]
    pub message_id: Option<Uuid>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::serde_iso8601")]
    pub created_at: DateTime,
    pub r#type: String,
    pub method: Option<String>,
    pub event_status: EventStatus,
    pub commitment_status: FlatCommitmentStatus,
    pub transaction_type: i32,
    pub transaction_idx: i32,
    pub message_idx: Option<i32>,
    pub event_idx: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub block_height: i64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Model {
    /// Returns the event timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at).to_rfc3339_string()
    }

    async fn transaction(&self, ctx: &Context<'_>) -> Result<Option<super::transactions::Model>> {
        let loader = ctx.data_unchecked::<DataLoader<EventTransactionDataLoader>>();
        Ok(loader.load_one(self.clone()).await?)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
